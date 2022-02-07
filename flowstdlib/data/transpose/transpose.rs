use flow_macro::flow_function;
use serde_json::Value;

#[flow_function]
#[allow(clippy::needless_range_loop)]
fn _transpose(inputs: &[Value]) -> (Option<Value>, RunAgain) {
    let mut output_matrix: Vec<Value> = vec![]; // vector of Value::Array - i.e. array of rows

    if let Some(matrix) = inputs[0].as_array() {
        let rows = matrix.len();

        if let Some(row) = matrix[0].as_array() {
            let cols = row.len();
            let mut new_row; // Vector of Value::Number - i.e. a row
            for new_row_num in 0..cols {
                new_row = Vec::with_capacity(rows);
                for new_col_num in 0..rows {
                    new_row.push(matrix[new_col_num][new_row_num].clone());
                }
                output_matrix.push(Value::Array(new_row));
            }
        }
    }

    (Some(Value::Array(output_matrix)), RUN_AGAIN)
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use serde_json::Value;
    use super::_transpose;

    #[test]
    fn transpose_empty() {
        let matrix = Value::Array(vec![Value::Array(vec![])]);

        let inputs = vec![matrix];

        let (result, _) = _transpose(&inputs);

        let new_matrix = result.expect("Could not get the value from the output");

        assert_eq!(new_matrix, Value::Array(vec!()));
    }

    #[test]
    fn transpose_1x1() {
        let row0 = json!([1]);
        let matrix = Value::Array(vec![row0]);

        let inputs = vec![matrix];

        let (result, _) = _transpose(&inputs);

        let new_matrix = result.expect("Could not get the value from the output");
        let new_row0 = new_matrix[0].clone();

        assert_eq!(new_row0, json!([1]));
    }

    #[test]
    fn transpose_2x2() {
        let row0 = json!([1, 2]);
        let row1 = json!([3, 4]);
        let matrix = Value::Array(vec![row0, row1]);

        let inputs = vec![matrix];

        let (result, _) = _transpose(&inputs);

        let new_matrix = result.expect("Could not get the value from the output");
        let new_row0 = new_matrix[0].clone();
        let new_row1 = new_matrix[1].clone();

        assert_eq!(new_row0, json!([1, 3]));
        assert_eq!(new_row1, json!([2, 4]));
    }

    #[test]
    fn transpose_2x3() {
        let row0 = json!([1, 2, 3]);
        let row1 = json!([4, 5, 6]);
        let matrix = Value::Array(vec![row0, row1]);

        let inputs = vec![matrix];

        let (result, _) = _transpose(&inputs);

        let new_matrix = result.expect("Could not get the value from the output");
        let new_row0 = new_matrix[0].clone();
        let new_row1 = new_matrix[1].clone();
        let new_row2 = new_matrix[2].clone();

        assert_eq!(new_row0, json!([1, 4]));
        assert_eq!(new_row1, json!([2, 5]));
        assert_eq!(new_row2, json!([3, 6]));
    }
}
