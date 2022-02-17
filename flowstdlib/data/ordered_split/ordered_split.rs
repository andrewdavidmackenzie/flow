use flowmacro::flow_function;
use serde_json::{json, Value};

#[flow_function]
fn _ordered_split(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    if inputs[0].is_null() {
        Ok((Some(Value::Null), RUN_AGAIN))
    } else {
        let string = inputs[0].as_str().ok_or("Could not get string")?;
        let separator = inputs[1].as_str().ok_or("Could not get separator")?;
        let parts: Vec<&str> = string.split(separator).collect::<Vec<&str>>();
        Ok((Some(json!(parts)), RUN_AGAIN))
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::_ordered_split;

    #[test]
    fn simple() {
        let string = json!("the quick brown fox jumped over the lazy dog");
        let separator = json!(" ");

        let (result, _) = _ordered_split(&[string, separator]).expect("_ordered_split() failed");

        let output = result.expect("Could not get the Value from the output");
        let array = output.as_array().expect("Could not get the Array from the output");

        assert_eq!(array.len(), 9);
    }
}
