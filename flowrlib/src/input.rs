#[cfg(feature = "debugger")]
use std::fmt;

use log::debug;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// An `Input` can be initialized in one of two ways with an `InputInitializer`
pub enum InputInitializer {
    /// A `ConstantInputInitializer` initializes an input "constantly".
    /// i.e. after each time the associated function is run
    Constant(ConstantInputInitializer),
    /// A `OneTimeInputInitializer` initializes an `Input` once - at start-up before any
    /// functions are run. Then it is not initialized again, unless a reset if done for debugging
    OneTime(OneTimeInputInitializer),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
/// A `ConstantInputInitializer` initializes an input "constantly".
pub struct OneTimeInputInitializer {
    /// `once` is the `Valuez that the `Input` will be initialized with once on start-up
    pub once: Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
/// A `OneTimeInputInitializer` initializes an `Input` once - at start-up
pub struct ConstantInputInitializer {
    /// `constant` will be the `Value` that the `Input` will be initized with before each
    /// time the `Function` is run.
    pub constant: Value,
}

#[derive(Deserialize, Serialize)]
/// An `Input` to a `Function`.
pub struct Input {
    #[serde(default = "default_depth", skip_serializing_if = "is_default_depth")]
    /// An `Input` can accept `depth` number of inputs before it is considered "full" and
    /// ready to be used by the associated `Function`
    depth: usize,
    #[serde(default = "default_initial_value", skip_serializing_if = "Option::is_none")]
    /// An optional `InputInitializer` associated with this input
    pub initializer: Option<InputInitializer>,
    #[serde(default = "default_is_array", skip_serializing_if = "is_not_array")]
    /// `is_array` is used for implicit Object to Array of Objects conversion
    pub is_array: bool,
    #[serde(skip)]
    received: Vec<Value>,
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

fn is_not_array(is_array: &bool) -> bool {
    !*is_array
}

fn default_is_array() -> bool { false }

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
    /// Create a new `Input` with an optional `InputInitializer`
    pub fn new(depth: usize, initial_value: &Option<InputInitializer>, is_array: bool) -> Self {
        Input {
            depth,
            initializer: initial_value.clone(),
            received: Vec::with_capacity(depth),
            is_array
        }
    }

    #[cfg(feature = "debugger")]
    /// reset the value of an `Input` - usually only used while debugging
    pub fn reset(&mut self) {
        self.received.clear();
    }

    /// Take 'depth' elements from the Input and leave the rest for the next time
    pub fn take(&mut self) -> Vec<Value> {
        self.received.drain(0..self.depth).collect()
    }

    /// Initialize an input with the InputInitializer if it has one.
    /// When called at start-up    it will initialize      if it's a OneTime or Constant initializer
    /// When called after start-up it will initialize only if it's a            Constant initializer
    pub fn init(&mut self, first_time: bool) -> bool {
        let input_value = match (first_time, &self.initializer) {
            (true, Some(InputInitializer::OneTime(one_time))) => Some(one_time.once.clone()),
            (_, Some(InputInitializer::Constant(constant))) => Some(constant.constant.clone()),
            (_, None) | (false, Some(InputInitializer::OneTime(_))) => None
        };

        match input_value {
            Some(value) => {
                debug!("\t\tInput initialized with '{:?}'", value);
                self.push(value);
                true
            }
            _ => false
        }
    }

    /// Add a value to this `Input`
    pub fn push(&mut self, value: Value) {
        self.received.push(value);
    }

    /// Return true if the `Input` is empty or false otherwise
    pub fn is_empty(&self) -> bool { self.received.is_empty() }

    /// Return true of the `Input` is "full" and it's values can be taken for executing the `Function`
    pub fn full(&self) -> bool {
        self.received.len() >= self.depth
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use serde_json::Value;

    use super::Input;

    #[test]
    fn no_inputs_initially() {
        let input = Input::new(1, &None, false);
        assert!(input.is_empty());
    }

    #[test]
    fn accepts_value() {
        let mut input = Input::new(1, &None, false);
        input.push(Value::Null);
        assert!(!input.is_empty());
    }

    #[test]
    fn gets_full() {
        let mut input = Input::new(1, &None, false);
        input.push(Value::Null);
        assert!(input.full());
    }

    #[test]
    fn take_empties() {
        let mut input = Input::new(1, &None, false);
        input.push(json!(10));
        assert!(!input.is_empty());
        input.take();
        assert!(input.is_empty());
    }

    #[cfg(feature = "debugger")]
    #[test]
    fn reset_empties() {
        let mut input = Input::new(1, &None, false);
        input.push(json!(10));
        assert!(!input.is_empty());
        input.reset();
        assert!(input.is_empty());
    }

    #[test]
    fn depth_works() {
        let mut input = Input::new(2, &None, false);
        input.push(json!(5));
        assert!(!input.full());
        input.push(json!(10));
        assert!(input.full());
        assert_eq!(input.take().len(), 2);
    }

    #[test]
    fn can_hold_more_than_depth() {
        let mut input = Input::new(2, &None, false);
        input.push(json!(5));
        input.push(json!(10));
        input.push(json!(15));
        input.push(json!(20));
        input.push(json!(25));
        assert!(input.full());
    }

    #[test]
    fn can_take_from_more_than_depth() {
        let mut input = Input::new(2, &None, false);
        input.push(json!(5));
        input.push(json!(10));
        input.push(json!(15));
        input.push(json!(20));
        input.push(json!(25));
        assert!(input.full());
        let mut next_set = input.take();
        assert_eq!(vec!(json!(5), json!(10)), next_set);
        assert!(input.full());
        next_set = input.take();
        assert_eq!(vec!(json!(15), json!(20)), next_set);
        assert!(!input.full());
    }
}