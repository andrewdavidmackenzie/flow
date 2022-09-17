#[cfg(feature = "debugger")]
use std::fmt;

use log::trace;
use serde_derive::{Deserialize, Serialize};
use serde_json::{json, Value};

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
    /// `array_order` defines how many levels of arrays of non-array values does the destination accept
    #[serde(
    default,
    skip_serializing_if = "is_default_array_order"
    )]
    array_order: i32,

    /// `generic` defines if the input accepts generic OBJECT_TYPEs
    #[serde(default, skip_serializing_if = "is_not_generic")]
    generic: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    // An optional `InputInitializer` associated with this input
    initializer: Option<InputInitializer>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    // An optional `InputInitializer` propagated from a flow input's initializer
    flow_initializer: Option<InputInitializer>,

    // The queue of values received so far as an ordered vector of entries,
    // with first will be at the head and last at the tail
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    received: Vec<Value>,

    #[cfg(feature = "debugger")]
    #[serde(default, skip_serializing_if = "String::is_empty")]
    name: Name
}

#[allow(clippy::trivially_copy_pass_by_ref)] // As this is imposed on us by serde
fn is_default_array_order(order: &i32) -> bool {
    *order == 0
}

#[allow(clippy::trivially_copy_pass_by_ref)] // As this is imposed on us by serde
fn is_not_generic(generic: &bool) -> bool {
    !*generic
}

impl From<&IO> for Input {
    fn from(io: &IO) -> Self {
        Input::new(
            #[cfg(feature = "debugger")]
            io.name(), io.datatypes()[0].array_order().unwrap_or(0),
            io.datatypes()[0].is_generic(),
            io.get_initializer().clone(),
        io.get_flow_initializer().clone())
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

impl Input {
    /// Create a new `Input` with an optional `InputInitializer`
    #[cfg(feature = "debugger")]
    pub fn new<S>(
        name: S,
        array_order: i32,
        generic: bool,
        initializer: Option<InputInitializer>,
        flow_initializer: Option<InputInitializer>,
    ) -> Self
    where S: Into<Name> {
        Input {
            name: name.into(),
            array_order,
            generic,
            initializer,
            flow_initializer,
            received: Vec::new(),
        }
    }

    /// Create a new `Input` with an optional `InputInitializer`
    #[cfg(not(feature = "debugger"))]
    pub fn new(
        initializer: Option<InputInitializer>,
        flow_initializer: Option<InputInitializer>) -> Self {
        Input {
            initializer,
            flow_initializer,
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

    /// return if this input is Generic
    pub(crate) fn is_generic(&self) -> bool {
        self.generic
    }

    /// Take the first element from the Input and return it.
    pub fn take(&mut self) -> Result<Value> {
        if self.received.is_empty() {
            bail!("Trying to take from an empty Input");
        }

        Ok(self.received.remove(0))
    }

    /// Initialize an input with the InputInitializer if it has one, either on the function directly
    /// or via a connection from a flow input
    /// When called at start-up    it will initialize      if it's a OneTime or Always initializer
    /// When called after start-up it will initialize only if it's a            Always initializer
    pub fn init(&mut self, first_time: bool, flow_idle: bool) -> bool {
        match (first_time, &self.initializer) {
            (true, Some(InputInitializer::Once(one_time))) => {
                self.send(one_time.clone());
                return true;
            },
            (_, Some(InputInitializer::Always(constant))) => {
                self.send(constant.clone());
                return true;
            },
            (_, _) => {},
        }

        match (first_time, &self.flow_initializer) {
            (true, Some(InputInitializer::Once(one_time))) => {
                self.send(one_time.clone());
                return true;
            },
            (true, Some(InputInitializer::Always(constant))) => {
                self.send(constant.clone());
                return true;
            },
            (_, _) => {},
        }

        if let (true, Some(InputInitializer::Always(constant))) = (flow_idle, &self.flow_initializer) {
            self.send(constant.clone());
            return true;
        }

        false
    }

    // return the array_order of this input
    fn array_order(&self) -> i32 {
        self.array_order
    }

    // Take a json data value and return the array order for it
    fn value_array_order(value: &Value) -> i32 {
        match value {
            Value::Array(array) if !array.is_empty() => {
                if let Some(value) = array.get(0) {
                    1 + Self::value_array_order(value)
                } else {
                    1
                }
            },
            Value::Array(array) if array.is_empty() => 1,
            _ => 0,
        }
    }

    /// Send a Value or array of Values to this input
    pub(crate) fn send(&mut self, value: Value) -> bool {
        if self.is_generic() {
            self.received.push(value);
        } else {
            match (Self::value_array_order(&value) - self.array_order(), &value) {
                (0, _) => self.received.push(value),
                (1, Value::Array(array)) => self.send_array(array.clone()),
                (2, Value::Array(array_2)) => {
                    for array in array_2.iter() {
                        if let Value::Array(sub_array) = array {
                            self.send_array(sub_array.clone())
                        }
                    }
                }
                (-1, _) => self.received.push(json!([value])),
                (-2, _) => self.received.push(json!([[value]])),
                _ => return false,
            }
        }
        true // a value was sent!
    }

    // Send an array of values to this `Input`, by sending them one by one
    fn send_array(&mut self, array: Vec<Value>)
    {
        for value in array {
            trace!("\t\t\tPushing array element '{value}'");
            self.received.push(value.clone());
        }
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
    fn no_inputs_initially() {
        let input = Input::new("", 0, false, None, None);
        assert!(input.is_empty());
    }

    #[test]
    fn take_from_empty_fails() {
        let mut input = Input::new("", 0, false,  None, None);
        assert!(input.take().is_err());
    }

    #[test]
    fn accepts_value() {
        let mut input = Input::new("", 0, false,  None, None);
        input.send(Value::Null);
        assert!(!input.is_empty());
    }

    #[test]
    fn accepts_array() {
        let mut input = Input::new("", 0, false,  None, None);
        input.send_array(vec![json!(5), json!(10), json!(15)]);
        assert!(!input.is_empty());
    }

    #[test]
    fn take_empties() {
        let mut input = Input::new("", 0, false,  None, None);
        input.send(json!(10));
        assert!(!input.is_empty());
        let _value = input.take().expect("Could not take the input value as expected");
        assert!(input.is_empty());
    }

    #[cfg(feature = "debugger")]
    #[test]
    fn reset_empties() {
        let mut input = Input::new("", 0,  false, None, None);
        input.send(json!(10));
        assert!(!input.is_empty());
        input.reset();
        assert!(input.is_empty());
    }

    #[test]
    fn test_array_order_0() {
        let value = json!(1);
        assert_eq!(Input::value_array_order(&value), 0);
    }

    #[test]
    fn test_array_order_1_empty_array() {
        let value = json!([]);
        assert_eq!(Input::value_array_order(&value), 1);
    }

    #[test]
    fn test_array_order_1() {
        let value = json!([1, 2, 3]);
        assert_eq!(Input::value_array_order(&value), 1);
    }

    #[test]
    fn test_array_order_2() {
        let value = json!([[1, 2, 3], [2, 3, 4]]);
        assert_eq!(Input::value_array_order(&value), 2);
    }
}
