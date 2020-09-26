use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::{json, Value};

#[derive(FlowImpl)]
/// Pass thru a value based on the index of an item in the stream of values
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "index"
/// source = "lib://flowstdlib/control/index"
/// ```
/// [[input]]
/// name = "value"
/// type = "Value"
///
/// [[input]]
/// name = "previous_value"
/// type = "Value"
///
/// [[input]]
/// name = "index"
/// type = "Number"
///
/// [[input]]
/// name = "select_index"
/// type = "Number"
///
/// [[output]]
/// name = "index"
/// type = "Number"
///
/// [[output]]
/// name = "selected_value"
/// type = "Value"
///
/// [[output]]
/// name = "previous_value"
/// type = "Value"
#[derive(Debug)]
pub struct Index;

impl Implementation for Index {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let value = inputs[0].clone();
        let previous_value = inputs[1].clone();
        let index = inputs[2].as_i64().unwrap();
        let select_index = inputs[3].as_i64().unwrap();

        let mut output_map = serde_json::Map::new();

        match select_index {
            // A 'select_index' value of -1 indicates to output the last value before the null
            -1 if value.is_null() => output_map.insert("selected_value".into(), json!(previous_value)),
            // If 'select_value' is not -1 then see if it matches the current index
            _ if select_index == index => output_map.insert("selected_value".into(), json!(value)),
            _ => output_map.insert("previous_value".into(), json!(value))
        };

        // Output the index of the 'previous_value" about to be output
        output_map.insert("index".into(), json!(index +1));

        (Some(Value::Object(output_map)), RUN_AGAIN)
    }
}