use crate::domain_registry::{DomainRegistry, SuspendedContext};
use crate::interpreter::{Interpreter, SliceResult};
use crate::persistence::SnapshotManager;
use crate::goal_engine::GoalEngine;
use crate::rng::DeterministicRng;
use ved_tracer::Tracer;

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub converged: bool,
    pub steps: usize,
    pub low_priority_executed: bool,
    pub warning_detected: bool,
    pub trace: Vec<String>,
}

pub struct Scheduler {
    registry: DomainRegistry,
    snapshot_mgr: Option<SnapshotManager>,
    rng: DeterministicRng,
    pub tracer: Tracer,
}

impl Scheduler {
    pub fn new(registry: DomainRegistry) -> Self {
        Self { 
            registry, 
            snapshot_mgr: None, 
            rng: DeterministicRng::new(0),
            tracer: Tracer::new(),
        }
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rng = DeterministicRng::new(seed);
        self
    }

    pub fn with_snapshots(mut self, mgr: SnapshotManager) -> Self {
        self.snapshot_mgr = Some(mgr);
        self
    }

    /// Run the simulation until all inboxes are empty.
    /// Returns a full deterministic trace of execution.
    pub fn execute_until_quiescent(&mut self, max_cycles: usize, slice_gas_limit: usize) -> ExecutionResult {
        let mut active = true;
        let mut cycle = 0;
        let mut trace = Vec::new();
        
        let mut res = ExecutionResult {
            converged: true,
            steps: 0,
            low_priority_executed: false,
            warning_detected: false,
            trace: Vec::new(),
        };

        trace.push("[Scheduler] Starting execution loop...".to_string());

        while active {
            if cycle >= max_cycles {
                trace.push(format!("[Scheduler] HALT: Max cycles {} reached. Stopping to prevent infinite loop.", max_cycles));
                res.converged = false;
                res.warning_detected = true;
                break;
            }
            active = false;
            cycle += 1;
            res.steps += 1;

            let mut outbox_all = Vec::new();

            let mut domains_with_weights: Vec<(String, u64)> = self.registry.instances.iter()
                .filter(|(_, v)| !v.is_quiescent)
                .map(|(k, v)| {
                // Heuristic dynamic urgency mapping
                let high_bonus = if !v.mailbox.high.is_empty() { 1000 } else { 0 };
                
                let oldest_ticket = v.mailbox.high.front().map(|m| m.clock)
                                        .unwrap_or_else(|| v.mailbox.normal.front().map(|m| m.clock).unwrap_or(cycle as u64));
                let aging_bonus = (cycle as u64).saturating_sub(oldest_ticket) * 10;
                
                let urgency = (v.schedule_weight as u64 * 100) + high_bonus + aging_bonus;
                
                (k.clone(), urgency)
            }).collect();
            // Deterministic sort: highest urgency first, fallback to lexicographical ID comparison
            domains_with_weights.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            let domain_names_sorted: Vec<String> = domains_with_weights.into_iter().map(|(k, _)| k).collect();
            

            for name in domain_names_sorted {
                let instance = self.registry.get_mut(&name).unwrap();

                // 1. Evaluate Goals
                // If a goal evaluates to true, we don't automatically stop the runtime, 
                // but we might log it or trigger a transition. For v0.1: just log if semantic goal is met.
                let mut goals_failed = false;
                for goal in &instance.bytecode.goals {
                    match GoalEngine::evaluate(goal, &instance.state, &instance.schema, slice_gas_limit) {
                        Ok(true) => {
                            self.tracer.record(cycle, &name, "GOAL_MET", &goal.name);
                            trace.push(format!("[Scheduler Cycle {}] Domain '{}' achieved GOAL: '{}'", cycle, name, goal.name));
                        }
                        Ok(false) => { 
                            goals_failed = true;
                            self.tracer.record(cycle, &name, "GOAL_FAILED", &goal.name);
                            trace.push(format!("[Scheduler Cycle {}] Domain '{}' GOAL FAILED: '{}' -> Scheduling recovery strategies", cycle, name, goal.name));
                            for strat in &goal.recovery_transitions {
                                // Idempotency Check: Do not enqueue if it is already queued.
                                let already_queued = instance.mailbox.high.iter().any(|m| &m.payload == strat) ||
                                                     instance.mailbox.normal.iter().any(|m| &m.payload == strat);
                                
                                if !already_queued {
                                    active = true;
                                    let recovery_msg = crate::messaging::Message {
                                        target_domain: name.clone(),
                                        payload: strat.clone(),
                                        priority: 1, // Enforce high priority
                                        clock: instance.logical_clock.tick,
                                    };
                                    let _ = instance.mailbox.push(recovery_msg);
                                }
                            }
                        }
                        Err(e) => {
                            self.tracer.record(cycle, &name, "GOAL_ERROR", &e);
                            trace.push(format!("[Scheduler Cycle {}] Domain '{}' goal error in '{}': {}", cycle, name, goal.name, e));
                        }
                    }
                }

                let mut active_trans = None;
                let mut start_pc = 0;
                let mut start_outbox = Vec::new();
                let mut is_resuming = false;
                let mut initial_registers = [0; 256];

                // Check for preempted threads first
                if let Some(suspended) = instance.suspended_context.take() {
                    active = true;
                    is_resuming = true;
                    trace.push(format!("[Scheduler Cycle {}] Domain '{}' resuming suspended transition '{}' at pc={}", cycle, name, suspended.transition_name, suspended.pc));
                    
                    for t in &instance.bytecode.transitions {
                        if t.name == suspended.transition_name {
                            active_trans = Some(t.clone());
                            break;
                        }
                    }
                    start_pc = suspended.pc;
                    start_outbox = suspended.outbox;
                    initial_registers = suspended.registers;
                } else if let Some(msg) = instance.mailbox.pop() {
                    active = true;
                    if msg.priority == 0 {
                        res.low_priority_executed = true;
                    }
                    instance.logical_clock.update(msg.clock);
                    instance.logical_clock.tick();
                    
                    self.tracer.record(cycle, &name, "PROCESS_MESSAGE", &msg.payload);
                    trace.push(format!("[Scheduler Cycle {}] Domain '{}' processing message: '{}' (Priority {}, Clock: {})", cycle, name, msg.payload, msg.priority, msg.clock));
                    
                    for t in &instance.bytecode.transitions {
                        if t.name == msg.payload {
                            active_trans = Some(t.clone());
                            break;
                        }
                    }
                    
                    if active_trans.is_none() {
                        trace.push(format!("[Scheduler Cycle {}] Domain '{}' dropped message '{}' (No matching transition)", cycle, name, msg.payload));
                    }
                }

                if let Some(trans) = active_trans {
                    let mut interpreter = Interpreter::with_state(instance.state.snapshot());
                    if is_resuming {
                        interpreter.registers = initial_registers;
                    }
                    
                    match interpreter.run_slice(&trans, &instance.schema, slice_gas_limit, start_pc, start_outbox) {
                        SliceResult::Completed(mut outbox) => {
                            instance.state = interpreter.state;
                            // Sort state keys for deterministic trace output
                            let state_keys = instance.state.keys_sorted();
                            
                            let state_str = state_keys.iter()
                                .map(|k| format!("\"{}\": {}", k, instance.state.get(k).unwrap()))
                                .collect::<Vec<_>>()
                                .join(", ");

                            self.tracer.record(cycle, &name, "STATE_MUTATED", &state_str);
                            trace.push(format!("[Scheduler Cycle {}] Domain '{}' state AFTER '{}': {{{}}}", cycle, name, trans.name, state_str));

                            // Assign logical clocks to outgoing messages
                            for out_msg in &mut outbox {
                                instance.logical_clock.tick();
                                out_msg.clock = instance.logical_clock.tick;
                            }

                            outbox_all.extend(outbox);
                        }
                        SliceResult::Suspended { pc, outbox } => {
                            trace.push(format!("[Scheduler Cycle {}] Domain '{}' transition '{}' exhausted gas slices, suspended at pc={}. Execution yielded.", cycle, name, trans.name, pc));
                            instance.suspended_context = Some(SuspendedContext {
                                transition_name: trans.name.clone(),
                                pc,
                                registers: interpreter.registers,
                                outbox,
                            });
                        }
                        SliceResult::Fault(e) => {
                            trace.push(format!("[Scheduler Cycle {}] Execution error in '{}': {}", cycle, name, e));
                        }
                    }
                }
                
                // Active Quiescence Check: Fall asleep if perfectly stable
                if instance.mailbox.is_empty() && instance.suspended_context.is_none() && !goals_failed {
                    instance.is_quiescent = true;
                    trace.push(format!("[Scheduler Cycle {}] Domain '{}' entered deterministic quiescent sleep state.", cycle, name));
                }
            }

            for msg in outbox_all {
                let msg_details = format!("Payload: {}, Target: {}", msg.payload, msg.target_domain);
                self.tracer.record(cycle, "SYSTEM", "ROUTING_MESSAGE", &msg_details);
                trace.push(format!("[Scheduler Cycle {}] Routing message -> [Target: {}, Payload: {}, Priority: {}, Clock: {}]", cycle, msg.target_domain, msg.payload, msg.priority, msg.clock));
                if let Err(e) = self.registry.route_message(msg) {
                    trace.push(format!("[Scheduler Cycle {}] ROUTING ERROR (Mailbox Full): {:?}", cycle, e));
                } else {
                    active = true;
                }
            }

            if let Some(mgr) = &self.snapshot_mgr {
                // Determine if we should save. We save if this cycle was active.
                if active {
                    if let Err(e) = mgr.save(cycle, &self.registry) {
                        trace.push(format!("[Scheduler Cycle {}] SNAPSHOT ERROR: {}", cycle, e));
                    } else {
                        trace.push(format!("[Scheduler Cycle {}] Snapshot saved successfully.", cycle));
                    }
                }
            }
        }

        trace.push("[Scheduler] Quiescent state reached. Halting.".to_string());
        res.trace = trace;
        res
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
                clock: 0,
            });
            let mut scheduler = Scheduler::new(registry);
            let trace = scheduler.execute_until_quiescent(1000, 1000).trace;
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
                clock: 0,
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
        mb.push(Message { target_domain: "A".to_string(), payload: "N".to_string(), priority: 0, clock: 0 }).unwrap();
        // Push 4 high
        for i in 0..4 {
            mb.push(Message { target_domain: "A".to_string(), payload: "H".to_string(), priority: 1, clock: i as u64 }).unwrap();
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
