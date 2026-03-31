mod common;
use ved_compiler::compile_source;
use ved_runtime::domain_registry::{DomainInstance, DomainRegistry};
use ved_runtime::scheduler::Scheduler;
use ved_runtime::persistence::{SnapshotData, SnapshotManager};
use ved_runtime::messaging::Message;
use std::fs;

#[test]
fn test_snapshot_integrity() {
    let source = r#"
        domain Counter {
            state { count: int }
            transition run {
                slice step { count = count + 1 }
            }
        }
    "#;

    let snapshot_file = "test_snapshot.json";
    let _ = fs::remove_file(snapshot_file);

    // Run to Cycle 5 natively
    {
        let program = compile_source(source).unwrap();
        let mut registry = DomainRegistry::new();
        
        let instance = DomainInstance::new(
            program.domains[0].name.clone(),
            program.domains[0].state_schema.clone(),
            program.domains[0].clone(),
        );
        registry.register(instance);
        
        registry.route_message(Message { target_domain: "Counter".into(), payload: "run".into(), priority: 0, clock: 0 }).unwrap();
        registry.route_message(Message { target_domain: "Counter".into(), payload: "run".into(), priority: 0, clock: 0 }).unwrap();

        let mgr = SnapshotManager::new(snapshot_file);
        let mut scheduler = Scheduler::new(registry).with_snapshots(mgr);
        let run_results = scheduler.execute_until_quiescent(10, 100);
        
        // Count should equal 2 (0 -> 1 -> 2) since we routed 2 messages dynamically natively.
        let reg = scheduler.get_registry();
        let end_state = reg.instances.get("Counter").unwrap().state.get("count").unwrap_or(0);
        assert_eq!(end_state, 2);
    } // Drops scheduler, forcing persistent snapshot flush naturally.

    // Reload from generic runtime snapshot entirely
    {
        let program = compile_source(source).unwrap();
        let mut registry = DomainRegistry::new();
        let instance = DomainInstance::new(
            program.domains[0].name.clone(),
            program.domains[0].state_schema.clone(),
            program.domains[0].clone(),
        );
        registry.register(instance);
        
        let mgr = SnapshotManager::new(snapshot_file);
        let snapshot_data = mgr.load().expect("Expected snapshot save bounds check!");
        mgr.restore_into(snapshot_data, &mut registry).unwrap();
        
        let restored_state = registry.instances.get("Counter").unwrap().state.get("count").unwrap_or(0);
        assert_eq!(restored_state, 2, "Snapshot integrity failed to reload isolated state parameters correctly!");
    }
    
    let _ = fs::remove_file(snapshot_file);
}
