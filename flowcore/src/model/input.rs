#[cfg(feature = "debugger")]
use std::fmt;

use log::debug;
use serde_derive::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::model::datatype::DataType;
use crate::model::input::InputInitializer::{Always, Once};
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

impl InputInitializer {
    /// Get the Value of the initializer
    pub fn get_value(&self) -> &Value {
        match self {
            Always(value) => value,
            Once(value) => value
        }
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
/// An [Input] to a [RuntimeFunction]
pub struct Input {
    #[cfg(feature = "debugger")]
    #[serde(default, skip_serializing_if = "String::is_empty")]
    name: Name,

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
            #[cfg(feature = "debugger")] io.name(),
            io.datatypes()[0].type_array_order(),
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
        array_order: i32,
        generic: bool,
        initializer: Option<InputInitializer>,
        flow_initializer: Option<InputInitializer>) -> Self {
        Input {
            array_order,
            generic,
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

    /// Return a reference to the initializer
    pub fn initializer(&self) -> &Option<InputInitializer> {
        &self.initializer
    }

    /// Return a reference to the flow initializer
    pub fn flow_initializer(&self) -> &Option<InputInitializer> {
        &self.flow_initializer
    }

    /// Initialize an input with the InputInitializer if it has one, either on the function directly
    /// or via a connection from a flow input
    /// When called at start-up    it will initialize      if it's a OneTime or Always initializer
    /// When called after start-up it will initialize only if it's a            Always initializer
    pub fn init(&mut self, first_time: bool, flow_idle: bool) -> bool {
        match (first_time, &self.initializer) {
            (true, Some(Once(one_time))) => {
                self.send(one_time.clone());
                return true;
            },
            (_, Some(Always(constant))) => {
                self.send(constant.clone());
                return true;
            },
            (_, _) => {},
        }

        match (first_time, &self.flow_initializer) {
            (true, Some(Once(one_time))) => {
                self.send(one_time.clone());
                return true;
            },
            (true, Some(Always(constant))) => {
                self.send(constant.clone());
                return true;
            },
            (_, _) => {},
        }

        if let (true, Some(Always(constant))) = (flow_idle, &self.flow_initializer) {
            self.send(constant.clone());
            return true;
        }

        false
    }

    // return the array_order of this input
    fn array_order(&self) -> i32 {
        self.array_order
    }

    /// Send a Value or array of Values to this input
    pub(crate) fn send(&mut self, value: Value) -> bool {
        if self.generic {
            self.received.push(value);
        } else {
            match (DataType::value_array_order(&value) - self.array_order(), &value) {
                (0, _) => self.received.push(value),
                (1, Value::Array(array)) => self.send_array_elements(array.clone()),
                (2, Value::Array(array_2)) => {
                    for array in array_2.iter() {
                        if let Value::Array(sub_array) = array {
                            self.send_array_elements(sub_array.clone())
                        }
                    }
                }
                (-1, _) => {
                    debug!("\t\tSending value '{value}' wrapped in an Array: '{}'",
                        json!([value]));
                    self.received.push(json!([value]))
                },
                (-2, _) => {
                    debug!("\t\tSending value '{value}' wrapped in an Array of Array: '{}'",
                        json!([[value]]));
                    self.received.push(json!([[value]]))
                },
                _ => return false,
            }
        }
        true // a value was sent!
    }

    // Send an array of values to this `Input`, by sending them one element at a time
    fn send_array_elements(&mut self, array: Vec<Value>) {
        debug!("\t\tSending Array as a series of Values");
        for value in array {
            debug!("\t\t\tSending array element as Value; '{value}'");
            self.received.push(value);
        }
    }

    /// Take the first element from the Input and return it. Could panic!
    pub fn take(&mut self) -> Value {
        self.received.remove(0)
    }

    /// Return the total number of values queued up in this input
    pub fn values_available(&self) -> usize {
        self.received.len()
    }

    /// Return true if there are no more values available from this input
    pub fn is_empty(&self) -> bool {
        self.values_available() == 0
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use serde_json::Value;

    use super::Input;

    #[test]
    fn no_inputs_initially() {
        let input = Input::new(#[cfg(feature = "debugger")] "", 0, false, None, None);
        assert!(input.is_empty());
    }

    #[test]
    #[should_panic]
    fn take_from_empty_fails() {
        let mut input = Input::new(#[cfg(feature = "debugger")] "", 0, false,  None, None);
        input.take();
    }

    #[test]
    fn accepts_null() {
        let mut input = Input::new(#[cfg(feature = "debugger")] "", 0, false,  None, None);
        input.send(Value::Null);
        assert!(!input.is_empty());
    }

    #[test]
    fn accepts_array() {
        let mut input = Input::new(#[cfg(feature = "debugger")] "", 0, false,  None, None);
        input.send_array_elements(vec![json!(5), json!(10), json!(15)]);
        assert!(!input.is_empty());
    }

    #[test]
    fn take_empties() {
        let mut input = Input::new(#[cfg(feature = "debugger")] "", 0, false,  None, None);
        input.send(json!(10));
        assert!(!input.is_empty());
        let _value = input.take();
        assert!(input.is_empty());
    }

    #[cfg(feature = "debugger")]
    #[test]
    fn reset_empties() {
        let mut input = Input::new(#[cfg(feature = "debugger")] "", 0,  false, None, None);
        input.send(json!(10));
        assert!(!input.is_empty());
        input.reset();
        assert!(input.is_empty());
    }
}
