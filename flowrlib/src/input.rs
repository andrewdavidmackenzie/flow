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
    pub initializer: Option<InputInitializer>,
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
            initializer: initial_value,
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

    /*
        Initialize an input with the InputInitializer if it has one.
        When called at start-up    it will initialize      if it's a OneTime or Constant initializer
        When called after start-up it will initialize only if it's a            Constant initializer
    */
    pub fn init(&mut self, first_time: bool) {
        let input_value = match (first_time, &self.initializer) {
            (true, Some(InputInitializer::OneTime(one_time))) => Some(one_time.once.clone()),
            (_, Some(InputInitializer::Constant(constant))) => Some(constant.constant.clone()),
            (_, None) | (false, Some(InputInitializer::OneTime(_))) => None
        };

        match input_value {
            Some(value) => {
                debug!("\t\tInput initialized with '{:?}'", value);
                self.push(value);
            }
            _ => {}
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