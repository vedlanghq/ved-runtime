use std::collections::HashMap;

/// Provides a strict, isolated memory boundary for a single Domain.
/// Ensures no shared mutable state, no accidental pointer aliasing,
/// and strictly enforces the domain's schema constraints.
#[derive(Debug, Clone, PartialEq)]
pub struct IsolatedState {
    memory: HashMap<String, i64>,
}

impl IsolatedState {
    /// Initialize memory zeroed-out matching the exact schema contour.
    pub fn new(schema: &[String]) -> Self {
        let mut memory = HashMap::new();
        for field in schema {
            memory.insert(field.clone(), 0);
        }
        Self { memory }
    }

    /// Read a value by value (copy), preventing pointer aliasing.
    pub fn get(&self, key: &str) -> Option<i64> {
        self.memory.get(key).copied()
    }

    /// Write a value. Strictly prevents appending state outside the explicit schema boundary,
    /// enforcing strict isolation and predictability.
    pub fn set(&mut self, key: &str, val: i64) -> Result<(), String> {
        if self.memory.contains_key(key) {
            self.memory.insert(key.to_string(), val);
            Ok(())
        } else {
            Err(format!("Memory violation: Attempted to write to undeclared schema field '{}'", key))
        }
    }

    /// Explicitly deep-clone the memory to hand a clean snapshot to an interpreter executing a slice,
    /// preventing any shared mutability.
    pub fn snapshot(&self) -> Self {
        self.clone()
    }

    /// Exposes a sorted list of keys for deterministic inspection (tests/logging).
    pub fn keys_sorted(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.memory.keys().cloned().collect();
        keys.sort();
        keys
    }
}
