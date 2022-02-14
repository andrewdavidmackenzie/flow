use flow_macro::flow_function;
use serde_json::json;
use serde_json::Value;

#[flow_function]
fn _zip(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let left = &inputs[0].as_array().ok_or("Could not get left array")?;
    let right = &inputs[1].as_array().ok_or("Could not get right array")?;
    let tuples = left.iter().zip(right.iter());
    let tuples_vec: Vec<(&Value, &Value)> = tuples.collect();
    Ok((Some(json!(tuples_vec)), RUN_AGAIN))
}