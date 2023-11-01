//! an lib to generate a unique id.

use std::{collections::HashSet, default};

/// a struct to generate a unique id.
pub struct IdGenerator {
    /// a set of used ids.
    used_ids: HashSet<u64>,
    /// a counter to generate a unique id.
    counter: u64,
}

impl default::Default for IdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl IdGenerator {
    /// a constructor to generate the struct to generate a unique id.
    pub fn new() -> Self {
        Self {
            used_ids: HashSet::new(),
            counter: 1,
        }
    }

    /// a constructor to generate the struct with default ID used.
    pub const fn with_used_ids(used_ids: HashSet<u64>) -> Self {
        Self {
            used_ids,
            counter: 1,
        }
    }
    /// a function to generate a unique i.
    pub fn generate_unique_id(&mut self) -> u64 {
        while self.used_ids.contains(&self.counter) {
            self.counter += 1;
        }
        self.used_ids.insert(self.counter);
        self.counter
    }
}
