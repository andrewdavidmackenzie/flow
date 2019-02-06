use serde_json::Value as JsonValue;
use std::mem::replace;
#[cfg(feature = "debugger")]
use std::fmt;

#[derive(Deserialize, Serialize)]
pub struct Input {
    #[serde(default = "default_depth", skip_serializing_if = "is_default")]
    depth: usize,
    #[serde(skip)]
    received: Vec<JsonValue>,
}

#[cfg(feature = "debugger")]
impl fmt::Display for Input {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.received.len() > 0 {
            for input_value in &self.received {
                write!(f, "'{}', ", input_value)?;
            }
        }
        write!(f, "")
    }
}

fn is_default(depth: &usize) -> bool {
    *depth == default_depth()
}

fn default_depth() -> usize {
    1
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

    pub fn overwrite(&mut self, value: JsonValue) {
        self.received[0] = value;
    }

    pub fn full(&self) -> bool {
        self.received.len() == self.depth
    }
}