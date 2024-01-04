use serde_json::{json, Value};

use flowcore::{RUN_AGAIN, RunAgain};
use flowcore::errors::*;
use flowmacro::flow_function;

#[flow_function]
fn _compose_matrix(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let mut new_matrix: Vec<Value> = vec![];
    let mut output_map = serde_json::Map::new();

    let element_to_add = inputs.first().ok_or("Could not get element")?.clone();

    let element_indexes = inputs.get(1).ok_or("Could not get element index")?
        .as_array().ok_or("Could not get element index array")?;

    let partial = inputs.get(2).ok_or("Could not get partial")?.as_array().ok_or("Could not get partial")?;
    let mut unwritten_cell_count = 0;

    // put element into the first null value we find, and only once
    for (row_index, row) in partial.iter().enumerate() {
        let mut new_row: Vec<Value> = vec!();
        let row_array = row.as_array().ok_or("Could not get row")?;
        for (column_index, element) in row_array.iter().enumerate() {
            let first_element_index = element_indexes.first().ok_or("Could not get index")?;
            let next_element_index = element_indexes.get(1).ok_or("Could not get index")?;
            if &row_index == first_element_index && &column_index == next_element_index {
                // This is the cell we want to write the element into
                new_row.push(element_to_add.clone());
            } else {
                // copy original element, whatever it is
                new_row.push(element.clone());
                if element.as_f64() == Some(0.0) {
                    unwritten_cell_count+= 1;
                }
            }
        }
        new_matrix.push(Value::Array(new_row));
    }

    if unwritten_cell_count == 0 {
        output_map.insert("matrix".into(), json!(new_matrix));
    } else {
        output_map.insert("partial".into(), json!(new_matrix));
    }

    Ok((Some(Value::Object(output_map)), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::_compose_matrix;

    #[test]
    fn compose_1_element() {
        let element = json!(42);
        let element_indexes = json!([0,1]);
        let partial = json!([[0.0, 0.0],[0.0,0.0]]);
        let inputs = vec![element, element_indexes, partial];

        let (result, _) = _compose_matrix(&inputs).expect("_compose_matrix() failed");

        let output = result.expect("Could not get the Value from the output");

        let matrix = output.pointer("/matrix");
        assert!(matrix.is_none());

        let partial = output.pointer("/partial");
        assert_eq!(partial, Some(&json!([[0.0, 42],[0.0,0.0]])));
    }

    #[test]
    fn compose_full_matrix() {
        let mut partial = json!([[0.0, 0.0],[0.0,0.0]]);

        for (index, element) in [1, 2, 3, 4].iter().enumerate() {
            let element_indexes = json!([index / 2, index % 2]);
            let inputs = vec![json!(element), element_indexes, partial];
            let (result, _) = _compose_matrix(&inputs).expect("_compose_matrix() failed");
            let output = result.expect("Could not get the Value from the output");

            if let Some(matrix) = output.pointer("/matrix") {
                assert_eq!(matrix, &json!([[1,2],[3,4]]));
                return;
            }
            partial = output.pointer("/partial").expect("Could not get partial").clone();
        }

        panic!("Should not get this far");
    }
}
