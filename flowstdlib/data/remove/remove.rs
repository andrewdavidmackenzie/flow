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
    fn run(&self, inputs: &Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        // Inputs
        let value = &inputs[0][0];
        let input1 = &inputs[1][0];
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
    use serde_json::{Number, Value};

    #[test]
    fn remove_1() {
        let array: Vec<Value> = vec!(Value::Array(vec!(Value::Number(Number::from(1)),
                                                       Value::Number(Number::from(2)))));
        let value = vec!(Value::Number(Number::from(1)));

        let remover = super::Remove{};
        let (result, _) = remover.run(&vec!(value, array));

        assert_eq!(result.unwrap(), Value::Array(vec!(Value::Number(Number::from(2)))));
    }

    #[test]
    fn not_remove_3() {
        let array: Vec<Value> = vec!(Value::Array(vec!(Value::Number(Number::from(1)),
                                                       Value::Number(Number::from(2)))));
        let value = vec!(Value::Number(Number::from(3)));

        let remover = super::Remove{};
        let (result, _) = remover.run(&vec!(value, array));

        assert_eq!(result.unwrap(), Value::Array(vec!(Value::Number(Number::from(1)),
                                                      Value::Number(Number::from(2)))));
    }
}