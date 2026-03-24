use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use serde::{Deserialize, Serialize};

use crate::state::IsolatedState;
use crate::messaging::Message;
use crate::domain_registry::DomainRegistry;

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotData {
    pub cycle: usize,
    pub domains: HashMap<String, DomainSnapshot>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DomainSnapshot {
    pub state: IsolatedState,
    pub mailbox_high: Vec<Message>,
    pub mailbox_normal: Vec<Message>,
}

pub struct SnapshotManager {
    filepath: String,
}

impl SnapshotManager {
    pub fn new(filepath: &str) -> Self {
        Self {
            filepath: filepath.to_string(),
        }
    }

    /// Read the snapshot from disk, if it exists.
    pub fn load(&self) -> Result<SnapshotData, String> {
        let path = Path::new(&self.filepath);
        if !path.exists() {
            return Err("Snapshot file not found".to_string());
        }

        let file = File::open(path).map_err(|e| format!("Failed to open snapshot: {}", e))?;
        let reader = BufReader::new(file);
        let data: SnapshotData = serde_json::from_reader(reader).map_err(|e| format!("Failed to deserialize snapshot: {}", e))?;
        
        Ok(data)
    }

    /// Take a full snapshot of the registry and save it.
    pub fn save(&self, cycle: usize, registry: &DomainRegistry) -> Result<(), String> {
        let mut data = SnapshotData {
            cycle,
            domains: HashMap::new(),
        };

        for (name, instance) in &registry.instances {
            let domain_snap = DomainSnapshot {
                state: instance.state.clone(),
                mailbox_high: instance.mailbox.high.iter().cloned().collect(),
                mailbox_normal: instance.mailbox.normal.iter().cloned().collect(),
            };
            data.domains.insert(name.clone(), domain_snap);
        }

        let temp_path = format!("{}.tmp", self.filepath);
        {
            let file = File::create(&temp_path).map_err(|e| format!("Failed to create temporary snapshot: {}", e))?;
            let writer = BufWriter::new(file);
            serde_json::to_writer_pretty(writer, &data).map_err(|e| format!("Failed to serialize snapshot: {}", e))?;
        }

        std::fs::rename(&temp_path, &self.filepath).map_err(|e| format!("Failed to finalize snapshot file: {}", e))?;

        Ok(())
    }

    /// Safely apply a loaded snapshot to an existing registry.
    pub fn restore_into(&self, data: SnapshotData, registry: &mut DomainRegistry) -> Result<(), String> {
        for (name, domain_snap) in data.domains {
            if let Some(instance) = registry.get_mut(&name) {
                // Restore state
                instance.state = domain_snap.state;
                // Restore mailbox
                instance.mailbox.high = domain_snap.mailbox_high.into();
                instance.mailbox.normal = domain_snap.mailbox_normal.into();
            } else {
                return Err(format!("Snapshot contains unknown domain '{}'", name));
            }
        }
        Ok(())
    }
}
