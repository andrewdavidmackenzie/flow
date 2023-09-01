use serde_json::{json, Value};

use flowcore::{RUN_AGAIN, RunAgain};
use flowcore::errors::*;
use flowmacro::flow_function;

#[flow_function]
fn _multiply_row(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let mut product = 0;
    let mut output_map = serde_json::Map::new();

    let row0 = inputs[0].as_array().ok_or("Could not get row0")?;
    let row1 = inputs[1].as_array().ok_or("Could not get row1")?;

    for index in 0..row0.len() {
        if let Some(row0_entry) = row0[index].as_i64() {
            if let Some(row1_entry) = row1[index].as_i64() {
                product += row0_entry * row1_entry;
            }
        }
    }

    output_map.insert("product".into(), json!(product));

    Ok((Some(Value::Object(output_map)), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use serde_json::Value;

    use super::_multiply_row;

    #[test]
    fn multiply_row() {
        let row0 = Value::Array(vec![json!(1), json!(2)]);
        let row1 = Value::Array(vec![json!(3), json!(4)]);

        let inputs = vec![row0, row1];

        let (result, _) = _multiply_row(&inputs).expect("_multiply_row() failed");

        let output = result.expect("Could not get the Value from the output");

        assert_eq!(output.pointer("/product").expect("Could not get 'product' output"), &json!(11));
    }
}
