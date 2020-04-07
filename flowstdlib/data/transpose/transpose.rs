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
pub struct Transpose;

impl Implementation for Transpose {
    fn run(&self, inputs: &Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let matrix = inputs[0][0].as_array().unwrap();

        let rows = matrix.len();
        let cols = matrix[0].as_array().unwrap().len();

        let mut output_matrix: Vec<Value> = vec![]; // vector of Value::Array - i.e. array of rows
        let mut new_row; // Vector of Value::Number - i.e. a row
        for new_row_num in 0..cols {
            new_row = Vec::with_capacity(rows);
            for new_col_num in 0..rows {
                new_row.push(matrix[new_col_num][new_row_num].clone());
            }
            output_matrix.push(Value::Array(new_row));
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
    fn transpose_empty() {
        let matrix = Value::Array(vec!(Value::Array(vec!())));

        let inputs = vec!(vec!(matrix));

        let transposer = super::Transpose {};
        let (result, _) = transposer.run(&inputs);

        let new_matrix = result.unwrap();

        assert_eq!(new_matrix, Value::Array(vec!()));
    }

    #[test]
    fn transpose_1x1() {
        let row0 = Value::Array(vec!(json!(1)));
        let matrix = Value::Array(vec!(row0));

        let inputs = vec!(vec!(matrix));

        let transposer = super::Transpose {};
        let (result, _) = transposer.run(&inputs);

        let new_matrix = result.unwrap();
        let new_row0 = new_matrix[0].clone();

        assert_eq!(new_row0, Value::Array(vec!(Value::Number(Number::from(1)))));
    }

    #[test]
    fn transpose_2x2() {
        let row0 = Value::Array(vec!(json!(1), json!(2)));
        let row1 = Value::Array(vec!(json!(3), json!(4)));
        let matrix = Value::Array(vec!(row0, row1));

        let inputs = vec!(vec!(matrix));

        let transposer = super::Transpose {};
        let (result, _) = transposer.run(&inputs);

        let new_matrix = result.unwrap();
        let new_row0 = new_matrix[0].clone();
        let new_row1 = new_matrix[1].clone();

        assert_eq!(new_row0, Value::Array(vec!(Value::Number(Number::from(1)), Value::Number(Number::from(3)))));
        assert_eq!(new_row1, Value::Array(vec!(Value::Number(Number::from(2)), Value::Number(Number::from(4)))));
    }

    #[test]
    fn transpose_2x3() {
        let row0 = Value::Array(vec!(json!(1), json!(2), json!(3)));
        let row1 = Value::Array(vec!(json!(4), json!(5), json!(6)));
        let matrix = Value::Array(vec!(row0, row1));

        let inputs = vec!(vec!(matrix));

        let transposer = super::Transpose {};
        let (result, _) = transposer.run(&inputs);

        let new_matrix = result.unwrap();
        let new_row0 = new_matrix[0].clone();
        let new_row1 = new_matrix[1].clone();
        let new_row2 = new_matrix[2].clone();

        assert_eq!(new_row0, Value::Array(vec!(Value::Number(Number::from(1)), Value::Number(Number::from(4)))));
        assert_eq!(new_row1, Value::Array(vec!(Value::Number(Number::from(2)), Value::Number(Number::from(5)))));
        assert_eq!(new_row2, Value::Array(vec!(Value::Number(Number::from(3)), Value::Number(Number::from(6)))));
    }
}