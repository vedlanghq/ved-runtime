use rand::{RngExt, SeedableRng};
use rand::rngs::StdRng;

pub struct DeterministicRng {
    rng: StdRng,
}

impl DeterministicRng {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
        }
    }

    pub fn next_bool(&mut self) -> bool {
        self.rng.random_bool(0.5)
    }

    pub fn next_in_range(&mut self, max: usize) -> usize {
        if max == 0 {
            return 0;
        }
        self.rng.random_range(0..max)
    }
}
