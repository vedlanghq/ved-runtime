use std::collections::HashMap;
use ved_ir::bytecode::DomainBytecode;
use crate::state::IsolatedState;
use crate::messaging::{Message, Mailbox};

pub struct DomainInstance {
    pub name: String,
    pub state: IsolatedState,
    pub schema: Vec<String>,
    pub bytecode: DomainBytecode,
    pub mailbox: Mailbox,
    pub schedule_weight: u8, // Higher weight = executes earlier in the cycle
}

impl DomainInstance {
    pub fn new(name: String, schema: Vec<String>, bytecode: DomainBytecode) -> Self {
        let state = IsolatedState::new(&schema);

        Self {
            name,
            state,
            schema,
            bytecode,
            mailbox: Mailbox::default(), // 100 capacity by default
            schedule_weight: 1,          // Default baseline priority
        }
    }

    pub fn with_weight(mut self, weight: u8) -> Self {
        self.schedule_weight = weight;
        self
    }
}

pub struct DomainRegistry {
    pub instances: HashMap<String, DomainInstance>,
}

impl DomainRegistry {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
        }
    }

    pub fn register(&mut self, instance: DomainInstance) {
        self.instances.insert(instance.name.clone(), instance);
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut DomainInstance> {
        self.instances.get_mut(name)
    }

    pub fn route_message(&mut self, msg: Message) -> Result<(), Message> {
        if let Some(instance) = self.instances.get_mut(&msg.target_domain) {
            instance.mailbox.push(msg)
        } else {
            println!("[Registry WARNING] Dropped message for unknown domain: {}", msg.target_domain);
            Err(msg)
        }
    }
}
