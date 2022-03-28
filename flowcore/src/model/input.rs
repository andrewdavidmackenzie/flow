use std::collections::BTreeMap;
#[cfg(feature = "debugger")]
use std::fmt;

use log::{debug, trace};
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

use crate::errors::*;
use crate::model::io::IO;
use crate::model::name::{HasName, Name};

#[derive(Clone, Debug, Serialize, PartialEq, Deserialize)]
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
/// An `Input` to a `RuntimeFunction`.
pub struct Input {
    #[serde(
        default = "default_initial_value",
        skip_serializing_if = "Option::is_none"
    )]
    // An optional `InputInitializer` associated with this input
    initializer: Option<InputInitializer>,

    // The prioritized queue of values received so far (priority, values)
    // priorities will be sparse, 0 the minimum and usize::MAX the maximum
    // values will be an ordered vector of entries, with first at the head and last at the tail
    #[serde(skip)]
    received: BTreeMap<usize, Vec<Value>>,

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
            write!(f, "'{}' ", self.name)?;
        }
        if let Some(initializer) = &self.initializer {
            write!(f, "Initializer: {:?}, ", initializer)?;
        }
        if self.received.is_empty() {
            write!(f, "Empty")?;
        } else {
            write!(f, "Received: ")?;
            for item in &self.received {
                write!(f, "{:?}, ", item.1)?;
            }
        }
        Ok(())
    }
}

fn default_initial_value() -> Option<InputInitializer> {
    None
}

impl Input {
    /// Create a new `Input` with an optional `InputInitializer`
    pub fn new<S>(
                #[cfg(feature = "debugger")]
                name: S,
               initial_value: &Option<InputInitializer>) -> Self
    where S: Into<Name> {
        Input {
            #[cfg(feature = "debugger")]
            name: name.into(),
            initializer: initial_value.clone(),
            received: BTreeMap::new(),
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

    /// Take the first element of the highest priority from the Input and return it.
    /// Remove a priority level if there are no values left with that priority
    pub fn take(&mut self) -> Result<Value> {
        if self.received.is_empty() {
            bail!("Trying to take from an empty Input");
        }

        #[allow(clippy::clone_on_copy)]
        let priority = self.received.keys().next()
            .ok_or("Priority Vector is empty")?.clone();
        let priority_vec = self.received.get_mut(&priority)
            .ok_or("Could not get the Priority Vector")?;
        let value = priority_vec.remove(0);         // remove the oldest element
        // if the vector of values for this priority is now empty, remove that priority entry
        if priority_vec.is_empty() {
            self.received.remove(&priority);
        }

        Ok(value)
    }

    /// Initialize an input with the InputInitializer if it has one.
    /// When called at start-up    it will initialize      if it's a OneTime or Constant initializer
    /// When called after start-up it will initialize only if it's a            Constant initializer
    pub fn init(&mut self, first_time: bool, io_number: usize) -> bool {
        if !self.received.is_empty() {
            return false;
        }

        let init_value = match (first_time, &self.initializer) {
            (true, Some(InputInitializer::Once(one_time))) => Some(one_time.clone()),
            (_, Some(InputInitializer::Always(constant))) => Some(constant.clone()),
            (_, None) | (false, Some(InputInitializer::Once(_))) => None,
        };

        match init_value {
            Some(value) => {
                debug!("\t\tInput #{} initialized with '{:?}'", io_number, value);
                self.push(0, value);
                true
            }
            _ => false,
        }
    }

    /// Add a `value` with `priority` to this `Input`
    pub fn push(&mut self, priority: usize, value: Value) {
        match self.received.get_mut(&priority) {
            Some(priority_vec) => {
                // add the value to the existing vector of values for this priority
                priority_vec.push(value);
            }
            None => {
                // create a new vec of values for this priority level and insert into the map
                self.received.insert(priority, vec!(value));
            }
        }
    }

    /// Add an array of values to this `Input`, by pushing them one by one
    pub fn push_array<'a, I>(&mut self, priority: usize, iter: I)
    where
        I: Iterator<Item = &'a Value>,
    {
        for value in iter {
            trace!("\t\t\tPushing array element '{}'", value);
            self.push(priority, value.clone());
        }
    }

    /// Return the total number of values queued up, across all priorities, in this input
    pub fn count(&self) -> usize {
        self.received.values().into_iter().fold(0, |sum, vec| sum + vec.len())
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
        input.push(0, Value::Null);
        assert!(!input.is_empty());
    }

    #[test]
    fn accepts_array() {
        let mut input = Input::new("", &None);
        input.push_array(0, vec![json!(5), json!(10), json!(15)].iter());
        assert!(!input.is_empty());
    }

    #[test]
    fn take_empties() {
        let mut input = Input::new("", &None);
        input.push(0, json!(10));
        assert!(!input.is_empty());
        let _value = input.take().expect("Could not take the input value as expected");
        assert!(input.is_empty());
    }

    #[test]
    fn two_simple_priorities() {
        let mut input = Input::new("", &None);
        input.push(1, json!(1));
        input.push(0, json!(0));
        assert_eq!(json!(0), input.take().expect("Could not take() any value"));
        assert_eq!(json!(1), input.take().expect("Could not take() any value"));
        assert!(input.is_empty());
    }

    #[test]
    fn multiple_values_per_priority() {
        let mut input = Input::new("", &None);
        input.push(1, json!(2));
        input.push(0, json!(0));
        input.push(1, json!(3));
        input.push(0, json!(1));
        assert_eq!(4, input.count());
        assert_eq!(json!(0), input.take().expect("Could not take() any value"));
        assert_eq!(json!(1), input.take().expect("Could not take() any value"));
        assert_eq!(json!(2), input.take().expect("Could not take() any value"));
        assert_eq!(json!(3), input.take().expect("Could not take() any value"));
        assert!(input.is_empty());
    }

    #[cfg(feature = "debugger")]
    #[test]
    fn reset_empties() {
        let mut input = Input::new("", &None);
        input.push(0, json!(10));
        assert!(!input.is_empty());
        input.reset();
        assert!(input.is_empty());
    }
}
