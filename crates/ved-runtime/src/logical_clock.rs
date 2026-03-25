use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct LogicalClock {
    pub tick: u64,
}

impl LogicalClock {
    pub fn new() -> Self {
        Self { tick: 0 }
    }

    pub fn tick(&mut self) -> u64 {
        self.tick += 1;
        self.tick
    }

    pub fn update(&mut self, incoming: u64) {
        if incoming >= self.tick {
            self.tick = incoming + 1;
        }
    }
}
