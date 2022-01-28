use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RUN_AGAIN, RunAgain};
use serde_json::Value;

#[derive(FlowImpl)]
/// Takes a value on it's input and sends the same value `factor` times in an array output
#[derive(Debug)]
pub struct Duplicate;

impl Implementation for Duplicate {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let value = &inputs[0];

        let mut output_array = vec![];

        if let Some(factor) = inputs[1].as_i64() {
            for _i in 0..factor {
                output_array.push(value.clone());
            }
        }

        (Some(Value::Array(output_array)), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use flowcore::Implementation;
    use serde_json::json;

    use super::Duplicate;

    #[test]
    fn duplicate_number() {
        let value = json!(42);
        let factor = json!(2);
        let inputs: Vec<serde_json::Value> = vec![value, factor];

        let duplicator = Duplicate {};
        let (output, _) = duplicator.run(&inputs);

        assert_eq!(output.expect("Could not get the Value from the output"), json!([42, 42]));
    }

    #[test]
    fn duplicate_row_of_numbers() {
        let value = json!([1, 2, 3]);
        let factor = json!(2);
        let inputs: Vec<serde_json::Value> = vec![value, factor];

        let duplicator = Duplicate {};
        let (output, _) = duplicator.run(&inputs);

        assert_eq!(output.expect("Could not get the Value from the output"), json!([[1, 2, 3], [1, 2, 3]]));
    }

    #[test]
    fn duplicate_matrix() {
        let value = json!([[1, 2, 3], [4, 5, 6], [7, 8, 9]]);
        let factor = json!(2);
        let inputs: Vec<serde_json::Value> = vec![value, factor];

        let duplicator = Duplicate {};
        let (output, _) = duplicator.run(&inputs);

        assert_eq!(
            output.expect("Could not get the Value from the output"),
            json!([
                [[1, 2, 3], [4, 5, 6], [7, 8, 9]],
                [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
            ])
        );
    }
}
