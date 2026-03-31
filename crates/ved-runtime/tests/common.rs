use ved_compiler::compile_source;
use ved_runtime::domain_registry::{DomainInstance, DomainRegistry};
use ved_runtime::scheduler::{Scheduler, ExecutionResult};
use ved_runtime::messaging::Message;

pub fn run_ved_program(source: &str, boot_messages: Vec<(&str, &str, u8)>, max_cycles: usize) -> ExecutionResult {
    let program = compile_source(source).unwrap();
    let mut registry = DomainRegistry::new();

    for domain in program.domains {
        let instance = DomainInstance::new(
            domain.name.clone(),
            domain.state_schema.clone(),
            domain.clone(),
        );
        registry.register(instance);
    }

    for (target, payload, priority) in boot_messages {
        let boot_msg = Message {
            target_domain: target.to_string(),
            payload: payload.to_string(),
            priority,
            clock: 0,
        };
        let _ = registry.route_message(boot_msg);
    }
    
    let mut scheduler = Scheduler::new(registry);
    scheduler.execute_until_quiescent(max_cycles, 1000)
}
