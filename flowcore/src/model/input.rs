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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    received: Vec<Value>,

    // How many values at the front of `received` are from internal connections
    #[serde(skip)]
    internal_count: usize,

    // Remaining elements from an array initializer for gradual delivery
    #[serde(skip)]
    pending_init_elements: Vec<Value>,
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
    where
        S: Into<Name>,
    {
        Input {
            name: name.into(),
            array_order,
            generic,
            initializer,
            flow_initializer,
            received: Vec::new(),
            internal_count: 0,
            pending_init_elements: Vec::new(),
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
            received: Vec::new(),
            internal_count: 0,
            pending_init_elements: Vec::new(),
        }
    }

    #[cfg(feature = "debugger")]
    /// Reset the an `Input` - clearing all received values (only used while debugging)
    pub fn reset(&mut self) {
        self.received.clear();
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

    /// Check if an initializer value should be delivered gradually (one element per idle cycle)
    /// Returns true if the value is an array with higher array order than this input accepts
    fn should_deliver_gradually(&self, value: &Value) -> bool {
        !self.generic && value.is_array() && DataType::value_array_order(value) > self.array_order()
    }

    /// Send one element from an array initializer, keeping the rest for later
    fn send_one_element(&mut self, value: &Value) -> bool {
        if let Value::Array(elements) = value {
            if let Some(first) = elements.first() {
                self.send(first.clone());
                self.pending_init_elements = elements.iter().skip(1).cloned().collect();
                return true;
            }
        }
        false
    }

    /// Initialize an input with the `InputInitializer` if it has one, either on the function
    /// directly or via a connection from a flow input
    /// When called at start-up    it will initialize      if it's a `Once` or `Always` initializer
    /// When called after start-up it will initialize only if it's a           `Always` initializer
    #[allow(clippy::match_same_arms)]
    pub fn init(&mut self, first_time: bool, flow_gone_idle: bool) -> bool {
        // Deliver pending elements from a previous gradual init
        if flow_gone_idle && !self.pending_init_elements.is_empty() {
            let next = self.pending_init_elements.remove(0);
            self.send(next);
            return true;
        }

        match (first_time, flow_gone_idle, &self.initializer) {
            (true, false, Some(Once(one_time))) => {
                if self.should_deliver_gradually(one_time) {
                    return self.send_one_element(&one_time.clone());
                }
                self.send(one_time.clone());
                return true;
            }
            (_, false, Some(Always(constant))) => {
                self.send(constant.clone());
                return true;
            }
            (_, _, _) => {}
        }

        // Flow initializers will only be applied if a function initializer has not already been
        // applied
        match (first_time, flow_gone_idle, &self.flow_initializer) {
            (true, _, Some(Once(one_time))) => {
                if self.should_deliver_gradually(one_time) {
                    return self.send_one_element(&one_time.clone());
                }
                self.send(one_time.clone());
                return true;
            }
            (true, _, Some(Always(constant))) => {
                self.send(constant.clone());
                return true;
            }
            (_, true, Some(Always(constant))) => {
                self.send(constant.clone());
                return true;
            }
            (_, _, _) => {}
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
            match (
                DataType::value_array_order(&value) - self.array_order(),
                &value,
            ) {
                (0, _) => self.received.push(value),
                (1, Value::Array(array)) => self.send_array_elements(array.clone()),
                (2, Value::Array(array_2)) => {
                    for array in array_2 {
                        if let Value::Array(sub_array) = array {
                            self.send_array_elements(sub_array.clone());
                        }
                    }
                }
                (-1, _) => {
                    debug!(
                        "\t\tSending value '{value}' wrapped in an Array: '{}'",
                        json!([value])
                    );
                    self.received.push(json!([value]));
                }
                (-2, _) => {
                    debug!(
                        "\t\tSending value '{value}' wrapped in an Array of Array: '{}'",
                        json!([[value]])
                    );
                    self.received.push(json!([[value]]));
                }
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

    /// Send a value from an internal connection — inserted before external values
    pub(crate) fn send_internal(&mut self, value: Value) -> bool {
        if self.generic {
            self.received.insert(self.internal_count, value);
            self.internal_count += 1;
        } else {
            match (
                DataType::value_array_order(&value) - self.array_order(),
                &value,
            ) {
                (0, _) => {
                    self.received.insert(self.internal_count, value);
                    self.internal_count += 1;
                }
                (1, Value::Array(array)) => {
                    for v in array {
                        self.received.insert(self.internal_count, v.clone());
                        self.internal_count += 1;
                    }
                }
                (2, Value::Array(array_2)) => {
                    for array in array_2 {
                        if let Value::Array(sub_array) = array {
                            for v in sub_array {
                                self.received.insert(self.internal_count, v.clone());
                                self.internal_count += 1;
                            }
                        }
                    }
                }
                (-1, _) => {
                    self.received.insert(self.internal_count, json!([value]));
                    self.internal_count += 1;
                }
                (-2, _) => {
                    self.received.insert(self.internal_count, json!([[value]]));
                    self.internal_count += 1;
                }
                _ => return false,
            }
        }
        true
    }

    /// Clear all internal values from this input, preserving external values
    pub fn clear_internal(&mut self) {
        if self.internal_count > 0 {
            self.received.drain(..self.internal_count);
            self.internal_count = 0;
        }
    }

    /// Take the first element from the Input and return it. Could panic!
    #[must_use]
    pub fn take(&mut self) -> Option<Value> {
        if self.received.is_empty() {
            return None;
        }

        if self.internal_count > 0 {
            self.internal_count -= 1;
        }
        Some(self.received.remove(0))
    }

    /// Return the total number of values queued up in this input
    #[must_use]
    pub fn values_available(&self) -> usize {
        self.received.len()
    }

    /// Return true if there are no more values available from this input
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values_available() == 0
    }

    /// Return true if this input has pending internal values
    #[must_use]
    pub fn has_internal(&self) -> bool {
        self.internal_count > 0
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use crate::model::input::InputInitializer::{Always, Once};
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
        input.send(Value::Null);
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
        input.send_array_elements(vec![json!(5), json!(10), json!(15)]);
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
        input.send(json!(10));
        assert!(!input.is_empty());
        let _value = input
            .take()
            .expect("Should have got a value from the input");
        assert!(input.is_empty());
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
        input.send(json!(10));
        assert!(!input.is_empty());
        input.reset();
        assert!(input.is_empty());
    }

    #[test]
    fn init_first_time_once() {
        let mut input = Input::new(
            #[cfg(feature = "debugger")]
            "",
            0,
            false,
            Some(Once(json!(1))),
            None,
        );

        input.init(true, false);

        assert_eq!(input.take(), Some(json!(1)));
        assert!(input.is_empty());
    }

    #[test]
    fn init_first_time_always() {
        let mut input = Input::new(
            #[cfg(feature = "debugger")]
            "",
            0,
            false,
            Some(Always(json!(1))),
            None,
        );

        input.init(true, false);

        assert_eq!(input.take(), Some(json!(1)));
        assert!(input.is_empty());
    }

    #[test]
    fn init_later_once() {
        let mut input = Input::new(
            #[cfg(feature = "debugger")]
            "",
            0,
            false,
            Some(Once(json!(1))),
            None,
        );

        input.init(true, false);

        assert_eq!(input.take(), Some(json!(1)));
        assert!(input.is_empty());

        input.init(false, false);

        assert!(input.is_empty());
    }

    #[test]
    fn init_later_always() {
        let mut input = Input::new(
            #[cfg(feature = "debugger")]
            "",
            0,
            false,
            Some(Always(json!(1))),
            None,
        );

        input.init(true, false);

        assert_eq!(input.take(), Some(json!(1)));
        assert!(input.is_empty());

        input.init(false, false);

        assert_eq!(input.take(), Some(json!(1)));
        assert!(input.is_empty());
    }

    #[test]
    fn gradual_init_array_on_number_input() {
        let mut input = Input::new(
            #[cfg(feature = "debugger")]
            "",
            0,
            false,
            Some(Once(json!([1, 2, 3]))),
            None,
        );

        // First init: sends only element 0
        input.init(true, false);
        assert_eq!(input.take(), Some(json!(1)));
        assert!(input.is_empty());

        // Idle cycle: sends element 1
        input.init(false, true);
        assert_eq!(input.take(), Some(json!(2)));
        assert!(input.is_empty());

        // Idle cycle: sends element 2
        input.init(false, true);
        assert_eq!(input.take(), Some(json!(3)));
        assert!(input.is_empty());

        // No more elements
        input.init(false, true);
        assert!(input.is_empty());
    }

    #[test]
    fn non_gradual_array_on_array_input() {
        // array/number input receiving array/number value — same order, send whole array
        let mut input = Input::new(
            #[cfg(feature = "debugger")]
            "",
            1,
            false,
            Some(Once(json!([1, 2, 3]))),
            None,
        );

        input.init(true, false);
        assert_eq!(input.take(), Some(json!([1, 2, 3])));
        assert!(input.is_empty());
    }
}
