use serde_json::{json, Value};

use flowcore::{RUN_AGAIN, RunAgain};
use flowcore::errors::*;
use flowmacro::flow_function;

#[flow_function]
#[allow(clippy::needless_range_loop)]
fn _transpose(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let mut output_matrix: Vec<Value> = vec![]; // vector of Value::Array - i.e. array of rows
    let mut col_indexes = vec![];
    let mut output_map = serde_json::Map::new();

    let matrix = inputs[0].as_array().ok_or("Could not get array")?;

    let rows = matrix.len();

    let row = matrix[0].as_array().ok_or("Could not get array")?;

    let cols = row.len();
    let mut new_row; // Vector of Value::Number - i.e. a row
    for new_row_num in 0..cols {
        new_row = Vec::with_capacity(rows);
        for new_col_num in 0..rows {
            new_row.push(matrix[new_col_num][new_row_num].clone());
        }
        output_matrix.push(Value::Array(new_row));
        col_indexes.push(new_row_num);
    }

    output_map.insert("matrix".into(), json!(output_matrix));
    output_map.insert("column_indexes".into(), json!(col_indexes));

    Ok((Some(Value::Object(output_map)), RUN_AGAIN))
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

        let (result, _) = _transpose(&inputs).expect("_transpose() failed");

        let output = result.expect("Could not get the Value from the output");

        assert_eq!(output.pointer("/matrix").expect("Could not get 'matrix' output"),
                   &Value::Array(vec!()));
    }

    #[test]
    fn transpose_1x1() {
        let row0 = json!([1]);
        let matrix = Value::Array(vec![row0]);

        let inputs = vec![matrix];

        let (result, _) = _transpose(&inputs).expect("_transpose() failed");

        let output = result.expect("Could not get the Value from the output");

        let new_matrix = output.pointer("/matrix").expect("Could not get 'matrix' output");
        let new_row0 = new_matrix[0].clone();

        assert_eq!(new_row0, json!([1]));
    }

    #[test]
    fn transpose_2x2() {
        let row0 = json!([1, 2]);
        let row1 = json!([3, 4]);
        let matrix = Value::Array(vec![row0, row1]);

        let inputs = vec![matrix];

        let (result, _) = _transpose(&inputs).expect("_transpose() failed");

        let output = result.expect("Could not get the Value from the output");

        let new_matrix = output.pointer("/matrix").expect("Could not get 'matrix' output");
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

        let (result, _) = _transpose(&inputs).expect("_transpose() failed");

        let output = result.expect("Could not get the Value from the output");

        let new_matrix = output.pointer("/matrix").expect("Could not get 'matrix' output");
        let new_row0 = new_matrix[0].clone();
        let new_row1 = new_matrix[1].clone();
        let new_row2 = new_matrix[2].clone();

        assert_eq!(new_row0, json!([1, 4]));
        assert_eq!(new_row1, json!([2, 5]));
        assert_eq!(new_row2, json!([3, 6]));
    }
}
