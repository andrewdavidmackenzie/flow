use serde_json::{json, Value};

use flowcore::{RUN_AGAIN, RunAgain};
use flowcore::errors::*;
use flowmacro::flow_function;

#[flow_function]
fn _compose_matrix(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let mut new_matrix: Vec<Value> = vec![];
    let mut output_map = serde_json::Map::new();

    let element_to_add = inputs[0].clone();
    let partial = inputs[1].as_array().ok_or("Could not get partial")?;
    let mut matrix_full = true;
    let mut element_added = false;

    // put element into the first null value we find, and only once
    for (_row_index, row) in partial.iter().enumerate() {
        let mut new_row: Vec<Value> = vec!();
        let row_array = row.as_array().ok_or("Could not get row")?;
        for (_column_index, element) in row_array.iter().enumerate() {
            if element_added {
                // copy original element, whatever it is
                new_row.push(element.clone());
                if element == &Value::Null {
                    matrix_full = false; // nulls remain after adding element
                }
            }
            else {
                if element == &Value::Null {
                    new_row.push(element_to_add.clone());
                    element_added = true;
                } else {
                    // copy original element, whatever it is
                    new_row.push(element.clone());
                }
            }
        }
        new_matrix.push(Value::Array(new_row));
    }

    if matrix_full {
        output_map.insert("matrix".into(), json!(new_matrix));
    } else {
        output_map.insert("partial".into(), json!(new_matrix));
    }

    Ok((Some(Value::Object(output_map)), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use serde_json::Value;

    use super::_compose_matrix;

    #[test]
    fn compose_1_element() {
        let partial = json!([[Value::Null,Value::Null],[Value::Null,Value::Null]]);
        let element = json!(1);
        let inputs = vec![element, partial];

        let (result, _) = _compose_matrix(&inputs).expect("_compose_matrix() failed");

        let output = result.expect("Could not get the Value from the output");

        let matrix = output.pointer("/matrix");
        assert!(matrix.is_none());

        let partial = output.pointer("/partial");
        assert!(partial.is_some());
        println!("partial = {:?}", partial);
    }

    #[test]
    fn compose_full_matrix() {
        let mut partial = json!([[Value::Null,Value::Null],[Value::Null,Value::Null]]);

        for element in [1, 2, 3, 4] {
            let inputs = vec![json!(element), partial];
            let (result, _) = _compose_matrix(&inputs).expect("_compose_matrix() failed");
            let output = result.expect("Could not get the Value from the output");

            if let Some(matrix) = output.pointer("/matrix") {
                assert_eq!(matrix, &json!([[1,2],[3,4]]));
                return;
            }
            partial = output.pointer("/partial").expect("Could not get partial").clone();
        }

        assert!(false, "Should not get this far");
    }
}
