#[cfg(feature = "debugger")]
use std::fmt;

use log::trace;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

use crate::errors::*;
use crate::model::io::IO;
#[cfg(feature = "debugger")]
use crate::model::name::HasName;
#[cfg(feature = "debugger")]
use crate::model::name::Name;

#[derive(Clone, Debug, Serialize, PartialEq, Eq, Deserialize)]
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

#[derive(Deserialize, Serialize, Clone, Debug)]
/// An `Input` to a `RuntimeFunction`
pub struct Input {
    #[serde(
        default = "default_initial_value",
        skip_serializing_if = "Option::is_none"
    )]
    // An optional `InputInitializer` associated with this input
    initializer: Option<InputInitializer>,

    // The queue of values received so far as an ordered vector of entries,
    // with first will be at the head and last at the tail
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    received: Vec<Value>,

    #[cfg(feature = "debugger")]
    #[serde(default, skip_serializing_if = "String::is_empty")]
    name: Name
}

impl From<&IO> for Input {
    fn from(io: &IO) -> Self {
        Input::new(
            #[cfg(feature = "debugger")]
            io.name(),
            io.get_initializer())
    }
}

#[cfg(feature = "debugger")]
impl fmt::Display for Input {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !self.name.is_empty() {
            write!(f, "({}) ", self.name)?;
        }
        if !self.received.is_empty() {
            write!(f, "{:?}", self.received)?;
        }
        Ok(())
    }
}

fn default_initial_value() -> Option<InputInitializer> {
    None
}

impl Input {
    /// Create a new `Input` with an optional `InputInitializer`
    #[cfg(feature = "debugger")]
    pub fn new<S>(
                name: S,
               initial_value: &Option<InputInitializer>) -> Self
    where S: Into<Name> {
        Input {
            name: name.into(),
            initializer: initial_value.clone(),
            received: Vec::new(),
        }
    }

    /// Create a new `Input` with an optional `InputInitializer`
    #[cfg(not(feature = "debugger"))]
    pub fn new(
        initial_value: &Option<InputInitializer>) -> Self {
        Input {
            initializer: initial_value.clone(),
            received: Vec::new(),
        }
    }

    #[cfg(feature = "debugger")]
    /// Reset the an `Input` - clearing all received values (only used while debugging)
    pub fn reset(&mut self) {
        self.received.clear();
    }

    #[cfg(feature = "debugger")]
    /// Return the name of the input
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Take the first element from the Input and return it.
    pub fn take(&mut self) -> Result<Value> {
        if self.received.is_empty() {
            bail!("Trying to take from an empty Input");
        }

        Ok(self.received.remove(0))
    }

    /// Initialize an input with the InputInitializer if it has one.
    /// When called at start-up    it will initialize      if it's a OneTime or Constant initializer
    /// When called after start-up it will initialize only if it's a            Constant initializer
    pub fn init(&mut self, first_time: bool, io_number: usize) -> bool {
        let init_value = match (first_time, &self.initializer) {
            (true, Some(InputInitializer::Once(one_time))) => Some(one_time.clone()),
            (_, Some(InputInitializer::Always(constant))) => Some(constant.clone()),
            (_, None) | (false, Some(InputInitializer::Once(_))) => None,
        };

        match init_value {
            Some(value) => {
                trace!("\t\tInputInitializer on Input:{} '{:?}'", io_number, value);
                self.push(value);
                true
            }
            _ => false,
        }
    }

    /// Add an array of values to this `Input`, by pushing them one by one
    pub fn push_array<'a, I>(&mut self, iter: I)
        where
            I: Iterator<Item = &'a Value>,
    {
        for value in iter {
            trace!("\t\t\tPushing array element '{}'", value);
            self.push(value.clone());
        }
    }

    /// Add a `value` to this `Input`
    pub fn push(&mut self, value: Value) {
        self.received.push(value);
    }

    /// Return the total number of values queued up, across all priorities, in this input
    pub fn count(&self) -> usize {
        self.received.len()
    }

    /// Return true if there are no more values available from this input
    pub fn is_empty(&self) -> bool {
        self.received.is_empty()
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
        let input = Input::new("", &None);
        assert!(input.is_empty());
    }

    #[test]
    fn take_from_empty_fails() {
        let mut input = Input::new("", &None);
        assert!(input.take().is_err());
    }

    #[test]
    fn accepts_value() {
        let mut input = Input::new("", &None);
        input.push(Value::Null);
        assert!(!input.is_empty());
    }

    #[test]
    fn accepts_array() {
        let mut input = Input::new("", &None);
        input.push_array(vec![json!(5), json!(10), json!(15)].iter());
        assert!(!input.is_empty());
    }

    #[test]
    fn take_empties() {
        let mut input = Input::new("", &None);
        input.push(json!(10));
        assert!(!input.is_empty());
        let _value = input.take().expect("Could not take the input value as expected");
        assert!(input.is_empty());
    }

    #[cfg(feature = "debugger")]
    #[test]
    fn reset_empties() {
        let mut input = Input::new("", &None);
        input.push(json!(10));
        assert!(!input.is_empty());
        input.reset();
        assert!(input.is_empty());
    }
}
