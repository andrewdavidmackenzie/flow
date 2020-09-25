use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// Remove a value from a vector of values
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "remove"
/// source = "lib://flowstdlib/data/remove"
/// ```
///
/// ## Input
/// name = "value"
/// * The value to remove from the array
///
/// ## Input
/// name = "array"
/// type = "Array"
/// * An array, to remove `value` from
///
/// ## Outputs
/// type = "Array"
/// * The resulting array
#[derive(Debug)]
pub struct Remove;

impl Implementation for Remove {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        // Inputs
        let value = &inputs[0];
        let input1 = &inputs[1];
        let mut input_array = input1.clone();
        let array = input_array.as_array_mut().unwrap();

        // Operation
        array.retain(|val| val != value);

        // Output
        let output = Value::Array(array.to_vec());

        (Some(output), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use flow_impl::Implementation;
    use serde_json::{json, Value};

    #[test]
    fn remove_1() {
        let array: Value = json!([1, 2]);
        let value = json!(1);

        let remover = super::Remove{};
        let (result, _) = remover.run(&[value, array]);

        assert_eq!(result.unwrap(), json!([2]));
    }

    #[test]
    fn remove_repeated_entry() {
        let array: Value = json!([1, 2, 2, 3, 4]);
        let value = json!(2);

        let remover = super::Remove{};
        let (result, _) = remover.run(&[value, array]);

        assert_eq!(result.unwrap(), json!([1, 3, 4]));
    }

    #[test]
    fn not_remove_3() {
        let array: Value = json!([1, 2]);
        let value = json!(3);

        let remover = super::Remove{};
        let (result, _) = remover.run(&[value, array]);

        assert_eq!(result.unwrap(), json!([1, 2]));
    }

    #[test]
    fn try_to_remove_from_empty_array() {
        let array: Value = json!([]);
        let value = json!(3);

        let remover = super::Remove{};
        let (result, _) = remover.run(&[value, array]);

        assert_eq!(result.unwrap(), json!([]));
    }

    #[test]
    fn try_to_remove_non_existant_entry() {
        let array: Value = json!([1, 2, 3, 5, 7, 8, 9]);
        let value = json!(6);

        let remover = super::Remove{};
        let (result, _) = remover.run(&[value, array]);

        assert_eq!(result.unwrap(), json!([1, 2, 3, 5, 7, 8, 9]));
    }
}