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
/// ## Input
/// name = "factor"
/// * duplication factor
///
/// ## Output
/// * Transposed matrix
#[derive(Debug)]
pub struct DuplicateRows;

impl Implementation for DuplicateRows {
    fn run(&self, inputs: &Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let matrix = inputs[0][0].as_array().unwrap();
        let factor = &inputs[1][0];
        let mut output_matrix: Vec<Value> = vec!();

        for row in matrix.iter() {
            for _i in 0..factor.as_i64().unwrap() {
                output_matrix.push(row.clone());
            }
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
    fn duplicate_2() {
        let row0 = Value::Array(vec!(json!(1), json!(2)));
        let row1 = Value::Array(vec!(json!(3), json!(4)));
        let matrix = Value::Array(vec!(row0, row1));

        let inputs = vec!(vec!(matrix), vec!(json!(2)));

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

    #[test]
    fn duplicate_3() {
        let row0 = Value::Array(vec!(json!(1), json!(2)));
        let row1 = Value::Array(vec!(json!(3), json!(4)));
        let matrix = Value::Array(vec!(row0, row1));

        let inputs = vec!(vec!(matrix), vec!(json!(3)));

        let duplicator = super::DuplicateRows {};
        let (result, _) = duplicator.run(&inputs);

        let new_matrix = result.unwrap();
        let new_row0 = new_matrix[0].clone();
        let new_row1 = new_matrix[1].clone();
        let new_row2 = new_matrix[2].clone();
        let new_row3 = new_matrix[3].clone();
        let new_row4 = new_matrix[4].clone();
        let new_row5 = new_matrix[5].clone();

        assert_eq!(new_row0, Value::Array(vec!(Value::Number(Number::from(1)), Value::Number(Number::from(2)))));
        assert_eq!(new_row1, Value::Array(vec!(Value::Number(Number::from(1)), Value::Number(Number::from(2)))));
        assert_eq!(new_row2, Value::Array(vec!(Value::Number(Number::from(1)), Value::Number(Number::from(2)))));
        assert_eq!(new_row3, Value::Array(vec!(Value::Number(Number::from(3)), Value::Number(Number::from(4)))));
        assert_eq!(new_row4, Value::Array(vec!(Value::Number(Number::from(3)), Value::Number(Number::from(4)))));
        assert_eq!(new_row5, Value::Array(vec!(Value::Number(Number::from(3)), Value::Number(Number::from(4)))));
    }
}