use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TraceEvent {
    pub cycle: usize,
    pub domain: String,
    pub action: String,
    pub details: String,
}

pub struct Tracer {
    events: Vec<TraceEvent>,
}

impl Tracer {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn record(&mut self, cycle: usize, domain: &str, action: &str, details: &str) {
        self.events.push(TraceEvent {
            cycle,
            domain: domain.to_string(),
            action: action.to_string(),
            details: details.to_string(),
        });
    }

    pub fn dump_json(&self) -> String {
        serde_json::to_string_pretty(&self.events).unwrap_or_else(|_| "[]".to_string())
    }

    pub fn format_trace_from_json(json_str: &str) -> Result<Vec<String>, String> {
        let events: Vec<TraceEvent> = serde_json::from_str(json_str).map_err(|e| e.to_string())?;
        let mut lines = Vec::new();
        for ev in events {
            lines.push(format!(
                "[Cycle {:03}] {:-15} | {:-18} | {}",
                ev.cycle, ev.domain, ev.action, ev.details
            ));
        }
        Ok(lines)
    }
}
