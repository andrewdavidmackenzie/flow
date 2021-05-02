use serde_json::{json, Value};

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

#[derive(FlowImpl)]
/// Sort an Array of Numbers
#[derive(Debug)]
pub struct Sort;

impl Implementation for Sort {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        if inputs[0].is_null() {
            (Some(Value::Null), RUN_AGAIN)
        } else if let Some(array_num) = inputs[0].as_array() {
            let mut array_of_numbers: Vec<Value> = array_num.clone();
            array_of_numbers.sort_by_key(|a| a.as_i64().unwrap_or(0));
            (Some(json!(array_of_numbers)), RUN_AGAIN)
        } else {
            (None, RUN_AGAIN)
        }
    }
}

#[cfg(test)]
mod test {
    use serde_json::{json, Value};

    use flowcore::Implementation;

    #[test]
    fn sort_null() {
        let sort = super::Sort {};
        let (result, _) = sort.run(&[Value::Null]);

        let output = result.expect("Could not get output value");
        assert_eq!(output, Value::Null);
    }

    #[test]
    fn sort_invalid() {
        let sort = super::Sort {};
        let (result, _) = sort.run(&[json!("Hello World")]);
        assert_eq!(result, None);
    }

    #[test]
    fn sort_one() {
        let sort = super::Sort {};
        let (result, _) = sort.run(&[json!([1])]);

        let output = result.expect("Could not get output value");
        assert_eq!(output, json!([1]));
    }

    #[test]
    fn sort_array() {
        let sort = super::Sort {};
        let (result, _) = sort.run(&[json!([7, 1, 4, 8, 3, 9])]);

        let output = result.expect("Could not get output value");
        assert_eq!(output, json!([1, 3, 4, 7, 8, 9]));
    }

    #[test]
    fn sort_array_repeats() {
        let sort = super::Sort {};
        let (result, _) = sort.run(&[json!([7, 1, 8, 4, 8, 3, 1, 9])]);

        let output = result.expect("Could not get output value");
        assert_eq!(output, json!([1, 1, 3, 4, 7, 8, 8, 9]));
    }
}
