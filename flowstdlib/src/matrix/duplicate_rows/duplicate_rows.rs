use serde_json::{json, Value};

use flowcore::{RUN_AGAIN, RunAgain};
use flowcore::errors::*;
use flowmacro::flow_function;

#[flow_function]
fn _duplicate_rows(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let mut output_matrix: Vec<Value> = vec![];
    let mut row_indexes = vec![];
    let mut output_map = serde_json::Map::new();

    let matrix = inputs.first().ok_or("Could not get matrix")?.as_array().ok_or("Could not get matrix")?;
    let factor = inputs.get(1).ok_or("Could not get factor")?.as_i64().ok_or("Could not get factor")?;

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

        assert_eq!(new_matrix.get(0).expect("Could not get [0]"), &json!([1, 2]));
        assert_eq!(new_matrix.get(1).expect("Could not get [1]"), &json!([1, 2]));
        assert_eq!(new_matrix.get(2).expect("Could not get [2]"), &json!([3, 4]));
        assert_eq!(new_matrix.get(3).expect("Could not get [3]"), &json!([3, 4]));

        let row_indexes = output.pointer("/row_indexes")
            .expect("Could not get 'row_indexes' output");
        assert_eq!(row_indexes.get(0).expect("Could not get [0]"), &json!(0));
        assert_eq!(row_indexes.get(1).expect("Could not get [1]"), &json!(0));
        assert_eq!(row_indexes.get(2).expect("Could not get [2]"), &json!(1));
        assert_eq!(row_indexes.get(3).expect("Could not get [3]"), &json!(1));
    }

    #[test]
    fn duplicate_3() {
        let matrix = json!([[1, 2], [3, 4]]);
        let duplication_factor = json!(3);

        let inputs = vec![matrix, duplication_factor];

        let (result, _) = _duplicate_rows(&inputs).expect("_duplicate_rows() failed");

        let output = result.expect("Could not get the Value from the output");

        let new_matrix = output.pointer("/matrix").expect("Could not get 'matrix' output");

        assert_eq!(new_matrix.get(0).expect("Could not get [0]"), &json!([1, 2]));
        assert_eq!(new_matrix.get(1).expect("Could not get [1]"), &json!([1, 2]));
        assert_eq!(new_matrix.get(2).expect("Could not get [2]"), &json!([1, 2]));
        assert_eq!(new_matrix.get(3).expect("Could not get [3]"), &json!([3, 4]));
        assert_eq!(new_matrix.get(4).expect("Could not get [4]"), &json!([3, 4]));
        assert_eq!(new_matrix.get(5).expect("Could not get [5]"), &json!([3, 4]));
    }
}
