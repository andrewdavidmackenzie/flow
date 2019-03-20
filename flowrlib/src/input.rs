use serde_json::Value as JsonValue;
use std::mem::replace;
#[cfg(feature = "debugger")]
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InputInitializer {
    Constant(ConstantInputInitializer),
    OneTime(OneTimeInputInitializer),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OneTimeInputInitializer {
    pub once: JsonValue,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConstantInputInitializer {
    pub constant: JsonValue,
}

#[derive(Deserialize, Serialize)]
pub struct Input {
    #[serde(default = "default_depth", skip_serializing_if = "is_default_depth")]
    depth: usize,
    #[serde(default = "default_initial_value", skip_serializing_if = "Option::is_none")]
    pub initial_value: Option<InputInitializer>,
    #[serde(skip)]
    received: Vec<JsonValue>,
}

#[cfg(feature = "debugger")]
impl fmt::Display for Input {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for input_value in &self.received {
            write!(f, "{}, ", input_value)?;
        }
        write!(f, "")
    }
}

fn is_default_depth(depth: &usize) -> bool {
    *depth == default_depth()
}

fn default_depth() -> usize {
    1
}

fn default_initial_value() -> Option<InputInitializer> {
    None
}

impl Input {
    pub fn new(depth: usize, initial_value: Option<InputInitializer>) -> Self {
        Input {
            depth,
            initial_value,
            received: Vec::with_capacity(depth),
        }
    }

    pub fn reset(&mut self) {
        self.received.clear();
    }

    pub fn read(&mut self) -> Vec<JsonValue> {
        self.received.clone()
    }

    pub fn take(&mut self) -> Vec<JsonValue> {
        replace(&mut self.received, Vec::with_capacity(self.depth))
    }

    pub fn init(&mut self) {
        if let Some(initial_value) = replace(&mut self.initial_value, None) {
            debug!("\t\tInput initialized '{:?}'", &initial_value);
            match &initial_value {
                InputInitializer::OneTime(one_time)  => self.push(one_time.once.clone()),
                InputInitializer::Constant(constant) => self.push(constant.constant.clone())
            }
        }
    }

    pub fn push(&mut self, value: JsonValue) {
        self.received.push(value);
    }

    pub fn overwrite(&mut self, value: JsonValue) {
        self.received[0] = value;
    }

    pub fn is_empty(&self) -> bool { self.received.is_empty() }

    pub fn full(&self) -> bool {
        self.received.len() == self.depth
    }
}

#[cfg(test)]
mod test {
    use super::Input;
    use serde_json::Value as JsonValue;

    #[test]
    fn no_inputs_initially() {
        let input = Input::new(1, None);
        assert!(input.is_empty());
    }

    #[test]
    fn accepts_value() {
        let mut input = Input::new(1, None);
        input.push(JsonValue::Null);
        assert!(!input.is_empty());
    }

    #[test]
    fn gets_full() {
        let mut input = Input::new(1, None);
        input.push(JsonValue::Null);
        assert!(input.full());
    }

    #[test]
    fn can_overwrite() {
        let mut input = Input::new(1, None);
        input.push(JsonValue::Null);
        input.overwrite(json!(10));
        assert_eq!(input.read(), vec!(json!(10)));
    }

    #[test]
    fn read_works() {
        let mut input = Input::new(1, None);
        input.push(json!(10));
        assert!(!input.is_empty());
        assert_eq!(input.read(), vec!(json!(10)));
    }

    #[test]
    fn take_empties() {
        let mut input = Input::new(1, None);
        input.push(json!(10));
        assert!(!input.is_empty());
        input.take();
        assert!(input.is_empty());
    }

    #[test]
    fn reset_empties() {
        let mut input = Input::new(1, None);
        input.push(json!(10));
        assert!(!input.is_empty());
        input.reset();
        assert!(input.is_empty());
    }

    #[test]
    fn depth_works() {
        let mut input = Input::new(2, None);
        input.push(json!(10));
        assert!(!input.full());
        input.push(json!(15));
        assert!(input.full());
        assert_eq!(input.take().len(), 2);
    }
}