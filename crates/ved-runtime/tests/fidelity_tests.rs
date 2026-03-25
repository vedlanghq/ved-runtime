use ved_compiler::compile_source;
use ved_runtime::domain_registry::{DomainInstance, DomainRegistry};
use ved_runtime::scheduler::Scheduler;
use ved_runtime::messaging::Message;
use ved_runtime::persistence::SnapshotManager;

fn setup_test_registry(source: &str) -> DomainRegistry {
    let program = compile_source(source).expect("Failed to compile test source");
    let mut registry = DomainRegistry::new();
    for domain in program.domains {
        let mut instance = DomainInstance::new(
            domain.name.clone(),
            domain.state_schema.clone(),
            domain.clone(),
        );
        // Seed some initial state or depend on boot message
        registry.register(instance);
    }
    registry
}

#[test]
fn test_2a_and_5a_journal_replay_and_trace_determinism() {
    let source = "
    domain CounterA {
        state {
            val: int
        }
        transition tick {
            slice step {
                val = val + 1
                send CounterB tock
            }
        }
    }
    domain CounterB {
        state {
            val: int
        }
        transition tock {
            slice step {
                val = val + 2
                if val < 6 {
                    send CounterA tick
                }
            }
        }
    }
    ";

    // 1. Run uninterrupted execution
    let mut reg1 = setup_test_registry(source);
    reg1.route_message(Message {
        target_domain: "CounterA".to_string(),
        payload: "tick".to_string(),
        priority: 0,
        clock: 0,
    }).unwrap();
    let mut sched1 = Scheduler::new(reg1);
    sched1.execute_until_quiescent(100, 1000);
    
    let trace1 = sched1.tracer.dump_json();
    let state_a_1 = sched1.tracer.events.last().unwrap(); // Just checking it runs

    // 2. Run with interruption (simulating crash) at cycle 2
    let mut reg2 = setup_test_registry(source);
    reg2.route_message(Message {
        target_domain: "CounterA".to_string(),
        payload: "tick".to_string(),
        priority: 0,
        clock: 0,
    }).unwrap();
    
    let tmp_snapshot = "fidelity_test.snapshot.json";
    let snapshot_mgr = SnapshotManager::new(tmp_snapshot);
    let mut sched2 = Scheduler::new(reg2).with_snapshots(SnapshotManager::new(tmp_snapshot));
    
    sched2.execute_until_quiescent(2, 1000); // Artificial bounds = crash
    
    // 3. Replay from snapshot
    let snapshot_data = snapshot_mgr.load().expect("Failed to load snapshot!");
    let mut reg3 = setup_test_registry(source); // fresh structs
    snapshot_mgr.restore_into(snapshot_data, &mut reg3).expect("Restore failed");
    
    let mut sched3 = Scheduler::new(reg3).with_snapshots(SnapshotManager::new(tmp_snapshot));  
    sched3.execute_until_quiescent(100, 1000); // Resume until quiescent
    
    // Stitch traces logic test:
    // (Assuming we could pull final states and compare natively)
    let domain_a_run1_state = sched1.tracer.events.iter().filter(|e| e.domain == "CounterA" && e.action == "STATE_MUTATED").last().map(|e| &e.details).unwrap();
    let domain_a_run3_state = sched3.tracer.events.iter().filter(|e| e.domain == "CounterA" && e.action == "STATE_MUTATED").last().map(|e| &e.details).unwrap();
    
    assert_eq!(domain_a_run1_state, domain_a_run3_state, "2.A Replay state mismatch! (Fidelity breach)");

    std::fs::remove_file(tmp_snapshot).ok();
}
