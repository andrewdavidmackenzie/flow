use serde_json::Value;

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

#[derive(FlowImpl)]
/// Remove a value from a vector of values
#[derive(Debug)]
pub struct Remove;

impl Implementation for Remove {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        // Inputs
        let value = &inputs[0];
        let input1 = &inputs[1];
        let mut input_array = input1.clone();

        let output = if let Some(array) = input_array.as_array_mut() {
            array.retain(|val| val != value);
            Value::Array(array.to_vec())
        } else {
            input_array
        };

        (Some(output), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use serde_json::{json, Value};

    use flowcore::Implementation;

    #[test]
    fn remove_1() {
        let array: Value = json!([1, 2]);
        let value = json!(1);

        let remover = super::Remove {};
        let (result, _) = remover.run(&[value, array]);

        assert_eq!(result.unwrap(), json!([2]));
    }

    #[test]
    fn remove_repeated_entry() {
        let array: Value = json!([1, 2, 2, 3, 4]);
        let value = json!(2);

        let remover = super::Remove {};
        let (result, _) = remover.run(&[value, array]);

        assert_eq!(result.unwrap(), json!([1, 3, 4]));
    }

    #[test]
    fn not_remove_3() {
        let array: Value = json!([1, 2]);
        let value = json!(3);

        let remover = super::Remove {};
        let (result, _) = remover.run(&[value, array]);

        assert_eq!(result.unwrap(), json!([1, 2]));
    }

    #[test]
    fn try_to_remove_from_empty_array() {
        let array: Value = json!([]);
        let value = json!(3);

        let remover = super::Remove {};
        let (result, _) = remover.run(&[value, array]);

        assert_eq!(result.unwrap(), json!([]));
    }

    #[test]
    fn try_to_remove_non_existent_entry() {
        let array: Value = json!([1, 2, 3, 5, 7, 8, 9]);
        let value = json!(6);

        let remover = super::Remove {};
        let (result, _) = remover.run(&[value, array]);

        assert_eq!(result.unwrap(), json!([1, 2, 3, 5, 7, 8, 9]));
    }
}
