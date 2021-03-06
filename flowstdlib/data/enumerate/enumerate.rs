use serde_json::{json, Value};

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

#[derive(FlowImpl)]
/// Enumerate the elements of an Array
#[derive(Debug)]
pub struct Enumerate;

impl Implementation for Enumerate {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let mut output_array: Vec<(usize, Value)> = vec![];

        if let Some(array) = inputs[0].as_array() {
            for (index, value) in array.iter().enumerate() {
                output_array.push((index, value.clone()));
            }
        }

        (Some(json!(output_array)), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use serde_json::{Number, Value};

    use flowcore::Implementation;

    #[test]
    fn enumerate() {
        let array = json!(["a", "b"]);

        let enumerator = super::Enumerate {};
        let (result, _) = enumerator.run(&vec![array]);

        let output = result.unwrap();
        let enumerated_array = output.as_array().unwrap();

        assert_eq!(enumerated_array.len(), 2);
        assert_eq!(
            enumerated_array[0],
            Value::Array(vec!(
                Value::Number(Number::from(0)),
                Value::String(String::from("a"))
            ))
        );
        assert_eq!(
            enumerated_array[1],
            Value::Array(vec!(
                Value::Number(Number::from(1)),
                Value::String(String::from("b"))
            ))
        );
    }

    #[test]
    fn enumerate_empty_array() {
        let array = json!([]);

        let enumerator = super::Enumerate {};
        let (result, _) = enumerator.run(&vec![array]);

        let output = result.unwrap();
        let enumerated_array = output.as_array().unwrap();

        assert_eq!(enumerated_array.len(), 0);
    }
}
