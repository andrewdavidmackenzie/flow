use serde_json::Value as JsonValue;
use std::mem::replace;

pub struct Input {
    depth: usize,
    received: Vec<JsonValue>,
}

impl Input {
    pub fn new(depth: usize) -> Self {
        Input {
            depth,
            received: Vec::with_capacity(depth)
        }
    }

    pub fn read(&mut self) -> Vec<JsonValue> {
        self.received.clone()
    }

    pub fn take(&mut self) -> Vec<JsonValue> {
        replace(&mut self.received, Vec::with_capacity(self.depth))
    }

    pub fn push(&mut self, value: JsonValue) {
        self.received.push(value);
    }

    pub fn full(&self) -> bool {
        self.received.len() == self.depth
    }
}