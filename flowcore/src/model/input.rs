use std::collections::HashMap;
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
#[must_use]
#[allow(clippy::module_name_repetitions)]
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
    #[must_use]
    pub fn get_value(&self) -> &Value {
        match self {
            Once(value) | Always(value) => value,
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq)]
struct InputValues {
    count: usize,
    prioritized_values_map: HashMap<usize, Vec<Value>>,
}

impl InputValues {
    #[must_use]
    fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn new() -> Self {
        Self {
            count: 0,
            prioritized_values_map: HashMap::new(),
        }
    }

    // `priority` will be used to take values out according to priority AND order of arrival
    fn push(&mut self, priority: usize, value: Value) {
        // if there is a list of values at this priority
        if let Some(values) = self.prioritized_values_map.get_mut(&priority) {
            values.push(value);
        } else {
            let values = vec![value];
            self.prioritized_values_map.insert(priority, values);
        }
        self.count += 1;
    }

    // Take the first value from the list of highest priority that has values
    #[must_use]
    fn take(&mut self) -> Option<Value> {
        if self.count == 0 {
            return None;
        }

        let mut priority = 0;

        loop {
            // if there is a list of values at this priority
            if let Some(value_vec) = self.prioritized_values_map.get_mut(&priority) {
                // take the first value from the list
                let next = value_vec.remove(0);
                self.count -= 1;
                // If the vector of values of this priority is now empty - remove the map entry
                if value_vec.is_empty() {
                    let _ = self.prioritized_values_map.remove(&priority);
                }
                return Some(next);
            }
            priority += 1;
        }
    }

    #[must_use]
    pub fn values_available(&self) -> usize {
        self.count
    }

    #[cfg(feature = "debugger")]
    pub fn reset(&mut self) {
        self.prioritized_values_map.clear();
        self.count = 0;
    }
}

#[cfg(feature = "debugger")]
impl fmt::Display for InputValues {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !self.is_empty() {
            write!(f, "{:?}", self.prioritized_values_map)?;
        }
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq, Debug)]
/// An [Input] to a `RuntimeFunction`
pub struct Input {
    #[cfg(feature = "debugger")]
    #[serde(default, skip_serializing_if = "String::is_empty")]
    name: Name,

    /// `array_order` defines how many levels of arrays of non-array values does the destination accept
    #[serde(default, skip_serializing_if = "is_default_array_order")]
    array_order: i32,

    /// `generic` defines if the input accepts generic object types
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
    #[serde(default, skip_serializing_if = "InputValues::is_empty")]
    received: InputValues,
}

#[allow(clippy::trivially_copy_pass_by_ref)] // As this is imposed on us by serde
fn is_default_array_order(order: &i32) -> bool {
    *order == 0
}

#[allow(clippy::trivially_copy_pass_by_ref)] // As this is imposed on us by serde
fn is_not_generic(generic: &bool) -> bool {
    !*generic
}

impl TryFrom<&IO> for Input {
    type Error = String;

    fn try_from(io: &IO) -> Result<Self, Self::Error> {
        let data_type = io.datatypes().first().ok_or("Could not get datatype")?;

        Ok(Input::new(
            #[cfg(feature = "debugger")]
            io.name(),
            data_type.type_array_order(),
            data_type.is_generic(),
            io.get_initializer().clone(),
            io.get_flow_initializer().clone(),
        ))
    }
}

#[cfg(feature = "debugger")]
impl fmt::Display for Input {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !self.name.is_empty() {
            write!(f, "({}) ", self.name)?;
        }
        write!(f, "{:?}", self.received)?;
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
    where
        S: Into<Name>,
    {
        Input {
            name: name.into(),
            array_order,
            generic,
            initializer,
            flow_initializer,
            received: InputValues::new(),
        }
    }

    /// Create a new `Input` with an optional `InputInitializer`
    #[cfg(not(feature = "debugger"))]
    #[must_use]
    pub fn new(
        array_order: i32,
        generic: bool,
        initializer: Option<InputInitializer>,
        flow_initializer: Option<InputInitializer>,
    ) -> Self {
        Input {
            array_order,
            generic,
            initializer,
            flow_initializer,
            received: InputValues::new(),
        }
    }

    #[cfg(feature = "debugger")]
    /// Reset the an `Input` - clearing all received values (only used while debugging)
    pub fn reset(&mut self) {
        self.received.reset();
    }

    #[cfg(feature = "debugger")]
    /// Return the name of the input
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return a reference to the initializer
    #[must_use]
    pub fn initializer(&self) -> &Option<InputInitializer> {
        &self.initializer
    }

    /// Return a reference to the flow initializer
    #[must_use]
    pub fn flow_initializer(&self) -> &Option<InputInitializer> {
        &self.flow_initializer
    }

    /// Initialize an input with the `InputInitializer` if it has one, either on the function directly
    /// or via a connection from a flow input
    /// When called at start-up    it will initialize      if it's a `Once` or `Always` initializer
    /// When called after start-up it will initialize only if it's a            `Always` initializer
    pub fn init(&mut self, first_time: bool, flow_idle: bool) -> bool {
        match (first_time, &self.initializer) {
            (true, Some(Once(one_time))) => {
                self.send(1, one_time.clone()); // TODO
                return true;
            }
            (_, Some(Always(constant))) => {
                self.send(1, constant.clone()); // TODO
                return true;
            }
            (_, _) => {}
        }

        match (first_time, &self.flow_initializer) {
            (true, Some(Once(one_time))) => {
                self.send(1, one_time.clone()); // TODO
                return true;
            }
            (true, Some(Always(constant))) => {
                self.send(1, constant.clone()); // TODO
                return true;
            }
            (_, _) => {}
        }

        if let (true, Some(Always(constant))) = (flow_idle, &self.flow_initializer) {
            self.send(1, constant.clone()); // TODO
            return true;
        }

        false
    }

    // return the array_order of this input
    fn array_order(&self) -> i32 {
        self.array_order
    }

    /// Send a Value or array of Values to this input
    pub(crate) fn send(&mut self, priority: usize, value: Value) -> bool {
        if self.generic {
            self.received.push(priority, value);
        } else {
            match (
                DataType::value_array_order(&value) - self.array_order(),
                &value,
            ) {
                (0, _) => self.received.push(priority, value),
                (1, Value::Array(array)) => self.send_array_elements(priority, array.clone()),
                (2, Value::Array(array_2)) => {
                    for array in array_2 {
                        if let Value::Array(sub_array) = array {
                            self.send_array_elements(priority, sub_array.clone());
                        }
                    }
                }
                (-1, _) => {
                    debug!(
                        "\t\tSending value '{value}' wrapped in an Array: '{}'",
                        json!([value])
                    );
                    self.received.push(priority, json!([value]));
                }
                (-2, _) => {
                    debug!(
                        "\t\tSending value '{value}' wrapped in an Array of Array: '{}'",
                        json!([[value]])
                    );
                    self.received.push(priority, json!([[value]]));
                }
                _ => return false,
            }
        }
        true // a value was sent!
    }

    // Send an array of values to this `Input`, by sending them one element at a time
    fn send_array_elements(&mut self, priority: usize, array: Vec<Value>) {
        debug!("\t\tSending Array as a series of Values");
        for value in array {
            debug!("\t\t\tSending array element as Value; '{value}'");
            self.received.push(priority, value);
        }
    }

    /// Take the first element from the Input and return it. Could panic!
    #[must_use]
    pub fn take(&mut self) -> Option<Value> {
        self.received.take()
    }

    /// Return the total number of values queued up in this input
    #[must_use]
    pub fn values_available(&self) -> usize {
        self.received.values_available()
    }

    /// Return true if there are no more values available to be taken from this input
    #[must_use]
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
        let input = Input::new(
            #[cfg(feature = "debugger")]
            "",
            0,
            false,
            None,
            None,
        );
        assert!(input.is_empty());
    }

    #[test]
    fn take_from_empty_fails() {
        let mut input = Input::new(
            #[cfg(feature = "debugger")]
            "",
            0,
            false,
            None,
            None,
        );
        assert!(input.take().is_none());
    }

    #[test]
    fn accepts_null() {
        let mut input = Input::new(
            #[cfg(feature = "debugger")]
            "",
            0,
            false,
            None,
            None,
        );
        input.send(1, Value::Null);
        assert!(!input.is_empty());
    }

    #[test]
    fn accepts_array() {
        let mut input = Input::new(
            #[cfg(feature = "debugger")]
            "",
            0,
            false,
            None,
            None,
        );
        input.send_array_elements(1, vec![json!(5), json!(10), json!(15)]);
        assert!(!input.is_empty());
    }

    #[test]
    fn take_empties() {
        let mut input = Input::new(
            #[cfg(feature = "debugger")]
            "",
            0,
            false,
            None,
            None,
        );
        input.send(1, json!(10));
        assert!(!input.is_empty());
        let _value = input
            .take()
            .expect("Should have got a value from the input");
        assert!(input.is_empty());
    }

    #[test]
    fn take_by_priority() {
        let mut input = Input::new(
            #[cfg(feature = "debugger")]
            "",
            0,
            false,
            None,
            None,
        );
        input.send(2, json!(20));
        input.send(0, json!(5));
        input.send(0, json!(6));
        input.send(1, json!(10));
        assert!(!input.is_empty());
        assert_eq!(input.take(), Some(json!(5)));
        assert_eq!(input.take(), Some(json!(6)));
        assert_eq!(input.take(), Some(json!(10)));
        assert_eq!(input.take(), Some(json!(20)));
        assert_eq!(input.take(), None);
    }

    #[cfg(feature = "debugger")]
    #[test]
    fn reset_empties() {
        let mut input = Input::new(
            #[cfg(feature = "debugger")]
            "",
            0,
            false,
            None,
            None,
        );
        input.send(1, json!(10));
        assert!(!input.is_empty());
        input.reset();
        assert!(input.is_empty());
    }
}
