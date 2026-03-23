use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub target_domain: String,
    pub payload: String,
    pub priority: u8, // 0 = Normal, 1 = High
}

pub struct Mailbox {
    pub high: VecDeque<Message>,
    pub normal: VecDeque<Message>,
    pub capacity: usize,
    skips: u8,
}

impl Default for Mailbox {
    fn default() -> Self {
        Self::new(100) // Default bound of 100 messages to prevent explosion
    }
}

impl Mailbox {
    pub fn new(capacity: usize) -> Self {
        Self {
            high: VecDeque::new(),
            normal: VecDeque::new(),
            capacity,
            skips: 0,
        }
    }

    pub fn push(&mut self, msg: Message) -> Result<(), Message> {
        if self.high.len() + self.normal.len() >= self.capacity {
            return Err(msg); // Backpressure constraint
        }
        if msg.priority > 0 {
            self.high.push_back(msg);
        } else {
            self.normal.push_back(msg);
        }
        Ok(())
    }

    pub fn pop(&mut self) -> Option<Message> {
        if self.high.is_empty() && self.normal.is_empty() {
            return None;
        }
        
        if self.high.is_empty() {
            self.skips = 0;
            return self.normal.pop_front();
        }
        
        if self.normal.is_empty() {
            return self.high.pop_front();
        }

        // Both queues have messages. Implement starvation control.
        // We allow up to 3 high-priority messages to jump the queue consecutively.
        // After 3 skips, we force a normal-priority message to ensure fairness.
        if self.skips >= 3 {
            self.skips = 0;
            self.normal.pop_front()
        } else {
            self.skips += 1;
            self.high.pop_front()
        }
    }
    
    pub fn is_empty(&self) -> bool {
        self.high.is_empty() && self.normal.is_empty()
    }
}
