use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// Takes a value on it's input and sends the same value `factor` times in an array output
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "duplicate"
/// source = "lib://flowstdlib/data/duplicate"
/// ```
///
///
/// ## Input
/// * `value` - the value to duplicate
///
/// ## Input
/// * `factor` - how many times to duplicate the value in the output
///
/// ## Outputs
/// * the array of duplicate values
#[derive(Debug)]
pub struct Duplicate;

impl Implementation for Duplicate {
    fn run(&self, inputs: &Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let value = &inputs[0][0];
        let factor = inputs[1][0].as_i64().unwrap();

        let mut output_array = vec!();

        for _i in 0..factor {
            output_array.push(value.clone());
        }

        (Some(Value::Array(output_array)), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use flow_impl::Implementation;
    use serde_json::json;

    use super::Duplicate;

    #[test]
    fn duplicate_number() {
        let value = vec!(json!(42));
        let factor = vec!(json!(2));
        let inputs: Vec<Vec<serde_json::Value>> = vec!(value, factor);

        let duplicator = Duplicate {};
        let (output, _) = duplicator.run(&inputs);

        assert_eq!(output.unwrap(), json!([42, 42]));
    }

    #[test]
    fn duplicate_row_of_numbers() {
        let value = vec!(json!([1, 2, 3]));
        let factor = vec!(json!(2));
        let inputs: Vec<Vec<serde_json::Value>> = vec!(value, factor);

        let duplicator = Duplicate {};
        let (output, _) = duplicator.run(&inputs);

        assert_eq!(output.unwrap(), json!([[1, 2, 3], [1, 2, 3]]));
    }

    #[test]
    fn duplicate_matrix() {
        let value = vec!(json!([[1, 2, 3], [4, 5, 6], [7, 8, 9]]));
        let factor = vec!(json!(2));
        let inputs: Vec<Vec<serde_json::Value>> = vec!(value, factor);

        let duplicator = Duplicate {};
        let (output, _) = duplicator.run(&inputs);

        assert_eq!(output.unwrap(), json!([[[1, 2, 3], [4, 5, 6], [7, 8, 9]], [[1, 2, 3], [4, 5, 6], [7, 8, 9]]]));
    }
}