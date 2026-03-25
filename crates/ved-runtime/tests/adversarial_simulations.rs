use ved_ir::bytecode::{OpCode, Constant, TransitionBytecode, DomainBytecode};
use ved_runtime::scheduler::Scheduler;
use ved_runtime::messaging::Message;
use ved_runtime::domain_registry::{DomainRegistry, DomainInstance};

#[test]
fn test_adversarial_infinite_loop_gas_limit() {
    let mut registry = DomainRegistry::new();

    let trans = TransitionBytecode {
        name: "infinite_loop".to_string(),
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
        state_schema: vec![],
        goals: vec![],
        transitions: vec![trans]
    };

    let instance = DomainInstance::new("Malicious".to_string(), vec![], domain);
    registry.register(instance);
    
    let msg = Message {
        target_domain: "Malicious".to_string(),
        payload: "infinite_loop".to_string(),
        priority: 0,
        clock: 0,
    };
    registry.route_message(msg).unwrap();

    let mut scheduler = Scheduler::new(registry);
    let trace = scheduler.execute_until_quiescent(10, 100);

    let found_gas_error = trace.iter().any(|t| t.contains("exhausted gas boundary"));
    assert!(found_gas_error, "Scheduler should catch the infinite loop and prevent system freeze");
}

#[test]
fn test_scheduler_starvation_fairness_flood() {
    let mut registry = DomainRegistry::new();

    // Add a simple domain to receive messages
    let trans1 = TransitionBytecode {
        name: "consume_normal".to_string(),
        constants: vec![],
        instructions: vec![OpCode::HaltSlice],
    };
    let trans2 = TransitionBytecode {
        name: "consume_high".to_string(),
        constants: vec![],
        instructions: vec![OpCode::HaltSlice],
    };

    let domain = DomainBytecode {
        name: "Victim".to_string(),
        state_schema: vec![],
        goals: vec![],
        transitions: vec![trans1, trans2]
    };

    registry.register(DomainInstance::new("Victim".to_string(), vec![], domain));

    // Flood the mailbox with 10 high-priority messages and 10 normal-priority messages
    for i in 0..10 {
        registry.route_message(Message {
            target_domain: "Victim".to_string(),
            payload: "consume_high".to_string(),
            priority: 1, // HIGH
            clock: i as u64,
        }).unwrap();
    }
    for i in 0..10 {
        registry.route_message(Message {
            target_domain: "Victim".to_string(),
            payload: "consume_normal".to_string(),
            priority: 0, // NORMAL
            clock: (i + 10) as u64,
        }).unwrap();
    }

    let mut scheduler = Scheduler::new(registry);
    
    // We expect the execution to interleave: H, H, H, N, H, H, H, N, etc...
    let trace = scheduler.execute_until_quiescent(20, 100);

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

#[test]
fn test_state_reconstruction_replay() {
    // Section 2.A: Journal replay fidelity
    use ved_runtime::persistence::SnapshotManager;
    
    let mut registry1 = DomainRegistry::new();
    let trans = TransitionBytecode {
        name: "inc".to_string(),
        constants: vec![Constant::Int(1)],
        instructions: vec![
            OpCode::LoadState { field_idx: 0, dest_reg: 0 },
            OpCode::LoadConst { const_idx: 0, dest_reg: 1 },
            OpCode::AddInt { r1: 0, r2: 1, dest: 2 },
            OpCode::StoreState { src_reg: 2, field_idx: 0 },
            OpCode::HaltSlice,
        ],
    };
    let mut domain = DomainBytecode {
        name: "Counter".to_string(),
        state_schema: vec!["val".to_string()],
        goals: vec![],
        transitions: vec![trans]
    };

    let mut instance1 = DomainInstance::new("Counter".to_string(), vec!["val".to_string()], domain.clone());
    instance1.state.set("val", 0).unwrap();
    registry1.register(instance1);
    
    // Seed msg
    registry1.route_message(Message {
        target_domain: "Counter".to_string(),
        payload: "inc".to_string(),
        priority: 0, clock: 0,
    }).unwrap();

    let mut dir = std::env::temp_dir();
    dir.push("ved_test_reconstruction.json");
    let test_file = dir.to_str().unwrap().to_string();
    let sm = SnapshotManager::new(&test_file);
    
    let mut sched1 = Scheduler::new(registry1).with_snapshots(sm);
    sched1.execute_until_quiescent(2, 100); // 2 cycles is enough to run 'inc'
    
    // At this point val should be 1, and snapshot should be saved.
    let snap_data = std::fs::read_to_string(&test_file).unwrap();
    assert!(snap_data.contains("\"val\":1"));
    
    // Reconstruct
    let mut registry2 = DomainRegistry::new();
    let mut instance2 = DomainInstance::new("Counter".to_string(), vec!["val".to_string()], domain);
    
    let sm_restore = SnapshotManager::new(&test_file);
    let loaded_snapshot = sm_restore.load().unwrap();
    // Use the persistence layer to restore
    sm_restore.restore_into(loaded_snapshot, &mut registry2).unwrap();
    
    // Ensure the domain was actually restored and state contains 1 
    let restored_instance = registry2.instances.get("Counter").unwrap();
    assert_eq!(restored_instance.state.get("val"), Some(1));
}
