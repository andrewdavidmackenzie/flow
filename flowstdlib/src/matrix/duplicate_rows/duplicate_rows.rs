use serde_json::{json, Value};

use flowcore::{RUN_AGAIN, RunAgain};
use flowcore::errors::*;
use flowmacro::flow_function;

#[flow_function]
fn _duplicate_rows(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let mut output_matrix: Vec<Value> = vec![];
    let mut row_indexes = vec![];
    let mut output_map = serde_json::Map::new();

    let matrix = inputs[0].as_array().ok_or("Could not get matrix")?;
    let factor = inputs[1].as_i64().ok_or("Could not get factor")?;

    for (row_index, row) in matrix.iter().enumerate() {
        for _i in 0..factor {
            output_matrix.push(row.clone());
            row_indexes.push(row_index)
        }
    }

    output_map.insert("matrix".into(), json!(output_matrix));
    output_map.insert("row_indexes".into(), json!(row_indexes));

    Ok((Some(Value::Object(output_map)), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::_duplicate_rows;

    #[test]
    fn duplicate_2() {
        let matrix = json!([[1, 2], [3, 4]]);
        let duplication_factor = json!(2);

        let inputs = vec![matrix, duplication_factor];

        let (result, _) = _duplicate_rows(&inputs).expect("_duplicate_rows() failed");

        let output = result.expect("Could not get the Value from the output");

        let new_matrix = output.pointer("/matrix").expect("Could not get 'matrix' output");

        assert_eq!(new_matrix[0], json!([1, 2]));
        assert_eq!(new_matrix[1], json!([1, 2]));
        assert_eq!(new_matrix[2], json!([3, 4]));
        assert_eq!(new_matrix[3], json!([3, 4]));

        let row_indexes = output.pointer("/row_indexes")
            .expect("Could not get 'row_indexes' output");
        assert_eq!(row_indexes[0], json!(0));
        assert_eq!(row_indexes[1], json!(0));
        assert_eq!(row_indexes[2], json!(1));
        assert_eq!(row_indexes[3], json!(1));
    }

    #[test]
    fn duplicate_3() {
        let matrix = json!([[1, 2], [3, 4]]);
        let duplication_factor = json!(3);

        let inputs = vec![matrix, duplication_factor];

        let (result, _) = _duplicate_rows(&inputs).expect("_duplicate_rows() failed");

        let output = result.expect("Could not get the Value from the output");

        let new_matrix = output.pointer("/matrix").expect("Could not get 'matrix' output");

        assert_eq!(new_matrix[0], json!([1, 2]));
        assert_eq!(new_matrix[1], json!([1, 2]));
        assert_eq!(new_matrix[2], json!([1, 2]));
        assert_eq!(new_matrix[3], json!([3, 4]));
        assert_eq!(new_matrix[4], json!([3, 4]));
        assert_eq!(new_matrix[5], json!([3, 4]));
    }
}
