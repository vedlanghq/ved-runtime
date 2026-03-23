use crate::domain_registry::DomainRegistry;
use crate::interpreter::Interpreter;

pub struct Scheduler {
    registry: DomainRegistry,
}

impl Scheduler {
    pub fn new(registry: DomainRegistry) -> Self {
        Self { registry }
    }

    /// Run the simulation until all inboxes are empty.
    /// Returns a full deterministic trace of execution.
    pub fn execute_until_quiescent(&mut self) -> Vec<String> {
        let mut active = true;
        let mut cycle = 0;
        let mut trace = Vec::new();

        trace.push("[Scheduler] Starting execution loop...".to_string());

        while active {
            active = false;
            cycle += 1;

            let mut outbox_all = Vec::new();

            let domain_names: Vec<String> = self.registry.instances.keys().cloned().collect();
            // Deterministic sort is absolutely critical here to prevent HashMap iteration randomness
            let mut domain_names_sorted = domain_names.clone();
            domain_names_sorted.sort();

            for name in domain_names_sorted {
                let instance = self.registry.get_mut(&name).unwrap();

                if let Some(msg) = instance.mailbox.pop() {
                    active = true;
                    trace.push(format!("[Scheduler Cycle {}] Domain '{}' processing message: '{}' (Priority {})", cycle, name, msg.payload, msg.priority));

                    let mut matched_trans = None;
                    for t in &instance.bytecode.transitions {
                        if t.name == msg.payload {
                            matched_trans = Some(t.clone());
                            break;
                        }
                    }

                    if let Some(trans) = matched_trans {
                        let mut interpreter = Interpreter::with_state(instance.state.snapshot());
                        match interpreter.run_slice(&trans, &instance.schema) {
                            Ok(outbox) => {
                                instance.state = interpreter.state;
                                // Sort state keys for deterministic trace output
                                let state_keys = instance.state.keys_sorted();
                                
                                let state_str = state_keys.iter()
                                    .map(|k| format!("\"{}\": {}", k, instance.state.get(k).unwrap()))
                                    .collect::<Vec<_>>()
                                    .join(", ");

                                trace.push(format!("[Scheduler Cycle {}] Domain '{}' state AFTER '{}': {{{}}}", cycle, name, trans.name, state_str));

                                outbox_all.extend(outbox);
                            }
                            Err(e) => {
                                trace.push(format!("[Scheduler Cycle {}] Execution error in '{}': {}", cycle, name, e));
                            }
                        }
                    } else {
                        trace.push(format!("[Scheduler Cycle {}] Domain '{}' dropped message '{}' (No matching transition)", cycle, name, msg.payload));
                    }
                }
            }

            for msg in outbox_all {
                trace.push(format!("[Scheduler Cycle {}] Routing message -> [Target: {}, Payload: {}, Priority: {}]", cycle, msg.target_domain, msg.payload, msg.priority));
                if let Err(e) = self.registry.route_message(msg) {
                    trace.push(format!("[Scheduler Cycle {}] ROUTING ERROR (Mailbox Full): {:?}", cycle, e));
                } else {
                    active = true;
                }
            }
        }

        trace.push("[Scheduler] Quiescent state reached. Halting.".to_string());
        trace
    }

    pub fn get_registry(&self) -> &DomainRegistry {
        &self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain_registry::{DomainInstance, DomainRegistry};
    use crate::messaging::{Message, Mailbox};
    use ved_ir::bytecode::{DomainBytecode, TransitionBytecode, OpCode, Constant};

    fn setup_registry() -> DomainRegistry {
        let mut registry = DomainRegistry::new();

        // Setup Producer
        let prod_trans = TransitionBytecode {
            name: "send_ping".to_string(),
            constants: vec![
                Constant::Int(1),
                Constant::String("Consumer".to_string()),
                Constant::String("receive_ping".to_string()),
            ],
            instructions: vec![
                OpCode::LoadConst { const_idx: 0, dest_reg: 0 },
                OpCode::StoreState { src_reg: 0, field_idx: 0 },
                OpCode::SendMsg { target_const_idx: 1, msg_const_idx: 2 },
                OpCode::HaltSlice,
            ],
        };
        
        let prod_bc = DomainBytecode {
            name: "Producer".to_string(),
            state_schema: vec!["sent".to_string()],
            transitions: vec![prod_trans],
            goals: vec![],
        };

        let prod_instance = DomainInstance::new(
            "Producer".to_string(),
            prod_bc.state_schema.clone(),
            prod_bc,
        );
        registry.register(prod_instance);

        // Setup Consumer
        let cons_trans = TransitionBytecode {
            name: "receive_ping".to_string(),
            constants: vec![
                Constant::Int(1),
            ],
            instructions: vec![
                OpCode::LoadState { field_idx: 0, dest_reg: 0 },
                OpCode::LoadConst { const_idx: 0, dest_reg: 1 },
                OpCode::AddInt { r1: 0, r2: 1, dest: 0 },
                OpCode::StoreState { src_reg: 0, field_idx: 0 },
                OpCode::HaltSlice,
            ],
        };

        let cons_bc = DomainBytecode {
            name: "Consumer".to_string(),
            state_schema: vec!["pings".to_string()],
            transitions: vec![cons_trans],
            goals: vec![],
        };

        let cons_instance = DomainInstance::new(
            "Consumer".to_string(),
            cons_bc.state_schema.clone(),
            cons_bc,
        );
        registry.register(cons_instance);

        registry
    }

    #[test]
    fn test_strict_determinism() {
        let mut first_trace = Vec::new();
        // Run the simulation 50 times to guarantee no accidental map traversals
        for i in 0..50 {
            let mut registry = setup_registry();
            let _ = registry.route_message(Message {
                target_domain: "Producer".to_string(),
                payload: "send_ping".to_string(),
                priority: 0,
            });
            let mut scheduler = Scheduler::new(registry);
            let trace = scheduler.execute_until_quiescent();
            if i == 0 {
                first_trace = trace;
            } else {
                assert_eq!(first_trace, trace, "Nondeterminism detected on {}!", i);
            }
        }
    }

    #[test]
    fn test_backpressure_mailbox_overflow() {
        let mut registry = setup_registry();
        // Manually constrain mailbox capacity for test
        registry.get_mut("Consumer").unwrap().mailbox = Mailbox::new(2);

        // Send 3 messages
        for i in 0..3 {
            let res = registry.route_message(Message {
                target_domain: "Consumer".to_string(),
                payload: "receive_ping".to_string(),
                priority: 0,
            });
            if i < 2 {
                assert!(res.is_ok(), "First 2 messages should push fine");
            } else {
                assert!(res.is_err(), "3rd message should overflow and return error");
            }
        }
    }

    #[test]
    fn test_priority_starvation_control() {
        let mut mb = Mailbox::new(10);
        
        // Push 1 normal
        mb.push(Message { target_domain: "A".to_string(), payload: "N".to_string(), priority: 0 }).unwrap();
        // Push 4 high
        for _ in 0..4 {
            mb.push(Message { target_domain: "A".to_string(), payload: "H".to_string(), priority: 1 }).unwrap();
        }

        // Expected pop order: High, High, High, Normal, High
        assert_eq!(mb.pop().unwrap().payload, "H");
        assert_eq!(mb.pop().unwrap().payload, "H");
        assert_eq!(mb.pop().unwrap().payload, "H");
        
        // Starvation control triggers:
        assert_eq!(mb.pop().unwrap().payload, "N"); // Normal inserted correctly to prevent starvation
        assert_eq!(mb.pop().unwrap().payload, "H"); 
        assert!(mb.pop().is_none());
    }
}
