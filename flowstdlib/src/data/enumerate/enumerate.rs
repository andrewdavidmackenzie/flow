use serde_json::{json, Value};

use flowcore::{RUN_AGAIN, RunAgain};
use flowcore::errors::Result;
use flowmacro::flow_function;

#[flow_function]
fn inner_enumerate(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let mut output_array: Vec<(usize, Value)> = vec![];

    let array = inputs.first().ok_or("Could not get array")?.as_array().ok_or("Could not get array")?;
    for (index, value) in array.iter().enumerate() {
        output_array.push((index, value.clone()));
    }

    Ok((Some(json!(output_array)), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::{Number, Value};
    use serde_json::json;

    use super::inner_enumerate;

    #[test]
    fn enumerate() {
        let array = json!(["a", "b"]);

        let (result, _) = inner_enumerate(&[array]).expect("_enumerate() failed");

        let output = result.expect("Could not get the Value from the output");
        let enumerated_array = output.as_array().expect("Could not get the Array from the output");

        assert_eq!(enumerated_array.len(), 2);
        assert_eq!(
            enumerated_array.first().expect("Could not get [0]"),
            &Value::Array(vec!(
                Value::Number(Number::from(0)),
                Value::String(String::from("a"))
            ))
        );
        assert_eq!(
            enumerated_array.get(1).expect("Could not get [1]"),
            &Value::Array(vec!(
                Value::Number(Number::from(1)),
                Value::String(String::from("b"))
            ))
        );
    }

    #[test]
    fn enumerate_empty_array() {
        let array = json!([]);

        let (result, _) = inner_enumerate(&[array]).expect("_enumerate() failed");

        let output = result.expect("Could not get the Value from the output");
        let enumerated_array = output.as_array().expect("Could not get the Value from the output");

        assert_eq!(enumerated_array.len(), 0);
    }
}
