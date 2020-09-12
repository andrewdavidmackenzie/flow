#[cfg(feature = "debugger")]
use std::fmt;

use log::{debug, trace};
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

use crate::errors::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
/// An `Input` can be initialized in one of two ways with an `InputInitializer`
pub enum InputInitializer {
    /// A `ConstantInputInitializer` initializes an input "constantly".
    /// i.e. after each time the associated function is run
    Always(Value),
    /// A `OneTimeInputInitializer` initializes an `Input` once - at start-up before any
    /// functions are run. Then it is not initialized again, unless a reset if done for debugging
    Once(Value),
}

#[derive(Deserialize, Serialize, Clone)]
/// An `Input` to a `Function`.
pub struct Input {
    #[serde(default = "default_initial_value", skip_serializing_if = "Option::is_none")]
    /// An optional `InputInitializer` associated with this input
    pub initializer: Option<InputInitializer>,
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

fn default_initial_value() -> Option<InputInitializer> {
    None
}

impl Input {
    /// Create a new `Input` with an optional `InputInitializer`
    pub fn new(initial_value: &Option<InputInitializer>) -> Self {
        Input {
            initializer: initial_value.clone(),
            received: Vec::new(),
        }
    }

    #[cfg(feature = "debugger")]
    /// reset the value of an `Input` - usually only used while debugging
    pub fn reset(&mut self) {
        self.received.clear();
    }

    /// Take first element from the Input and leave the rest for the next time
    pub fn take(&mut self) -> Result<Value> {
        if self.received.is_empty() {
            bail!("Trying to take from an empty input");
        }
        Ok(self.received.remove(0))
    }

    /// Initialize an input with the InputInitializer if it has one.
    /// When called at start-up    it will initialize      if it's a OneTime or Constant initializer
    /// When called after start-up it will initialize only if it's a            Constant initializer
    pub fn init(&mut self, first_time: bool, io_number: usize) -> bool {
        if self.full() {
            return false;
        }

        let init_value = match (first_time, &self.initializer) {
            (true, Some(InputInitializer::Once(one_time))) => Some(one_time.clone()),
            (_, Some(InputInitializer::Always(constant))) => Some(constant.clone()),
            (_, None) | (false, Some(InputInitializer::Once(_))) => None
        };

        match init_value {
            Some(value) => {
                debug!("\t\tInput #{} initialized with '{:?}'", io_number, value);
                self.push(value);
                true
            }
            _ => false
        }
    }

    /// Add a value to this `Input`
    pub fn push(&mut self, value: Value) {
        self.received.push(value);
        // if let Some(max) = self.depth {
        //     if self.received.len() > max {
        //         self.received.drain(0..1); // remove head to reduce number back to depth
        //     }
        // }
    }

    /// Add an array of values to this `Input`, by pushing them one by one
    pub fn push_array<'a, I>(&mut self, iter: I) where I: Iterator<Item=&'a Value> {
        for value in iter {
            trace!("\t\t\tPushing array element '{}'", value);
            self.received.push(value.clone());
        }
    }

    /// Return true if the `Input` is empty or false otherwise
    pub fn is_empty(&self) -> bool { self.received.is_empty() }

    /// Return true if the `Input` is "full" and it's values can be taken for executing the `Function`
    pub fn full(&self) -> bool {
        !self.received.is_empty()
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use serde_json::Value;

    use super::Input;

    #[test]
    fn default_initial_value_is_none() {
        assert!(super::default_initial_value().is_none());
    }

    #[test]
    fn no_inputs_initially() {
        let input = Input::new(&None);
        assert!(input.is_empty());
    }

    #[test]
    fn accepts_value() {
        let mut input = Input::new(&None);
        input.push(Value::Null);
        assert!(!input.is_empty());
    }

    #[test]
    fn accepts_array() {
        let mut input = Input::new(&None);
        input.push_array(vec!(json!(5), json!(10), json!(15)).iter());
        assert!(!input.is_empty());
    }

    #[test]
    fn gets_full() {
        let mut input = Input::new(&None);
        input.push(Value::Null);
        assert!(input.full());
    }

    #[test]
    fn take_empties() {
        let mut input = Input::new(&None);
        input.push(json!(10));
        assert!(!input.is_empty());
        let value = input.take().unwrap();
        assert_eq!(value, json!(10));
        assert!(input.is_empty());
    }

    #[cfg(feature = "debugger")]
    #[test]
    fn reset_empties() {
        let mut input = Input::new(&None);
        input.push(json!(10));
        assert!(!input.is_empty());
        input.reset();
        assert!(input.is_empty());
    }
}