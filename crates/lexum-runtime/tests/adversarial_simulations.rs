use Lexum_ir::bytecode::{OpCode, Constant, TransitionBytecode, DomainBytecode};
use Lexum_runtime::scheduler::Scheduler;
use Lexum_runtime::messaging::Message;
use Lexum_runtime::domain_registry::{DomainRegistry, DomainInstance};

#[test]
fn test_adversarial_infinite_loop_gas_limit() {
    let mut registry = DomainRegistry::new();

    let trans = TransitionBytecode {
        name: "infinite_loop".to_string(),
        required_capabilities: vec![],
        scope: None,
        constants: vec![Constant::Int(1)],
        instructions: vec![
            // pc = 0
            OpCode::LoadConst { const_idx: 0, dest_reg: 0 },
            // pc = 1: jump to 1
            OpCode::Jump { target_offset: 1 },
        ]
    };

    let domain = DomainBytecode {
        name: "Malicious".to_string(),
        capability_manifest: vec![],
        scope: None,
        state_schema: vec![],
        goals: vec![],
        invariants: vec![],
        transitions: vec![trans]
    };

    let instance = DomainInstance::new("Malicious".to_string(), vec![], domain);
    registry.register(instance);
    
    let msg = Message { id: "".into(), source_domain: "SYSTEM".into(),
        target_domain: "Malicious".to_string(),
        payload: "infinite_loop".to_string(),
        priority: 0,
        clock: 0,
    };
    registry.route_message(msg).unwrap();

    let mut scheduler = Scheduler::new(registry);
    let trace = scheduler.execute_until_quiescent(10, 100);

    let found_gas_error = trace.trace.iter().any(|t| t.contains("exhausted gas slices, suspended at pc="));
    assert!(found_gas_error, "Scheduler should catch the infinite loop and prevent system freeze");
}

#[test]
fn test_scheduler_starvation_fairness_flood() {
    let mut registry = DomainRegistry::new();

    // Add a simple domain to receive messages
    let trans1 = TransitionBytecode {
        name: "consume_normal".to_string(),
        required_capabilities: vec![],
        scope: None,
        constants: vec![],
        instructions: vec![OpCode::HaltSlice],
    };
    let trans2 = TransitionBytecode {
        name: "consume_high".to_string(),
        required_capabilities: vec![],
        scope: None,
        constants: vec![],
        instructions: vec![OpCode::HaltSlice],
    };

    let domain = DomainBytecode {
        name: "Victim".to_string(),
        capability_manifest: vec![],
        scope: None,
        state_schema: vec![],
        goals: vec![],
        invariants: vec![],
        transitions: vec![trans1, trans2]
    };

    registry.register(DomainInstance::new("Victim".to_string(), vec![], domain));

    // Flood the mailbox with 10 high-priority messages and 10 normal-priority messages
    for i in 0..10 {
        registry.route_message(Message { id: "".into(), source_domain: "SYSTEM".into(),
            target_domain: "Victim".to_string(),
            payload: "consume_high".to_string(),
            priority: 1, // HIGH
            clock: i as u64,
        }).unwrap();
    }
    for i in 0..10 {
        registry.route_message(Message { id: "".into(), source_domain: "SYSTEM".into(),
            target_domain: "Victim".to_string(),
            payload: "consume_normal".to_string(),
            priority: 0, // NORMAL
            clock: (i + 10) as u64,
        }).unwrap();
    }

    let mut scheduler = Scheduler::new(registry);
    
    // We expect the execution to interleave: H, H, H, N, H, H, H, N, etc...
    let trace = scheduler.execute_until_quiescent(20, 100).trace;

    let mut _high_count = 0;
    let mut normal_count = 0;
    for t in trace {
        if t.contains("AFTER 'consume_high'") {
            _high_count += 1;
        } else if t.contains("AFTER 'consume_normal'") {
            // Fairness guarantees normal isn't completely starved
            normal_count += 1;
        }
    }
    assert!(normal_count > 0, "Normal priority messages must not be starved!");
}
