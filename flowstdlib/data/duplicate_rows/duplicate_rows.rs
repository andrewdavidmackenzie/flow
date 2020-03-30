use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// Transpose a matricies rows and columns
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "transpose"
/// source = "lib://flowstdlib/data/transpose"
/// ```
///
/// ## Input
/// * Input matrix
///
/// ## Output
/// * Transposed matrix
#[derive(Debug)]
pub struct DuplicateRows;

impl Implementation for DuplicateRows {
    fn run(&self, inputs: &Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let matrix = inputs[0][0].as_array().unwrap();
        let mut output_matrix: Vec<Value> = vec!();

        for row in matrix.into_iter() {
            output_matrix.push(row.clone());
            output_matrix.push(row.clone());
        }

        (Some(Value::Array(output_matrix)), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use flow_impl::Implementation;
    use serde_json::{Number, Value};
    use serde_json::json;

    #[test]
    fn duplicate() {
        let row0 = Value::Array(vec!(json!(1), json!(2)));
        let row1 = Value::Array(vec!(json!(3), json!(4)));
        let matrix = Value::Array(vec!(row0, row1));

        let inputs = vec!(vec!(matrix));

        let duplicator = super::DuplicateRows {};
        let (result, _) = duplicator.run(&inputs);

        let new_matrix = result.unwrap();
        let new_row0 = new_matrix[0].clone();
        let new_row1 = new_matrix[1].clone();
        let new_row2 = new_matrix[2].clone();
        let new_row3 = new_matrix[3].clone();

        assert_eq!(new_row0, Value::Array(vec!(Value::Number(Number::from(1)), Value::Number(Number::from(2)))));
        assert_eq!(new_row1, Value::Array(vec!(Value::Number(Number::from(1)), Value::Number(Number::from(2)))));
        assert_eq!(new_row2, Value::Array(vec!(Value::Number(Number::from(3)), Value::Number(Number::from(4)))));
        assert_eq!(new_row3, Value::Array(vec!(Value::Number(Number::from(3)), Value::Number(Number::from(4)))));
    }
}