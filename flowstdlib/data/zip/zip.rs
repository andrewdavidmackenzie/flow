use serde_json::json;
use serde_json::Value;

use flow_macro::flow_function;

#[flow_function]
fn _zip(inputs: &[Value]) -> (Option<Value>, RunAgain) {
    if let Some(left) = &inputs[0].as_array() {
        if let Some(right) = &inputs[1].as_array() {
            let tuples = left.iter().zip(right.iter());
            let tuples_vec: Vec<(&Value, &Value)> = tuples.collect();
            return (Some(json!(tuples_vec)), RUN_AGAIN);
        }
    }

    (None, RUN_AGAIN)
}