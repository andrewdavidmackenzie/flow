use serde_json::{json, Value};

use flowcore::{RUN_AGAIN, RunAgain};
use flowcore::errors::*;
use flowmacro::flow_function;

#[flow_function]
fn _multiply_row(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let mut product = 0;
    let mut output_map = serde_json::Map::new();

    let a = inputs[0].as_array().ok_or("Could not get a")?;
    let a_index = inputs[1].as_u64();
    let b = inputs[2].as_array().ok_or("Could not get b")?;
    let b_index = inputs[3].as_u64();

    for index in 0..a.len() {
        if let Some(row0_entry) = a[index].as_i64() {
            if let Some(row1_entry) = b[index].as_i64() {
                product += row0_entry * row1_entry;
            }
        }
    }

    output_map.insert("product".into(), json!(product));
    output_map.insert("a_b_index".into(), json!([a_index, b_index]));

    Ok((Some(Value::Object(output_map)), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::_multiply_row;

    #[test]
    fn multiply_row() {
        let a = json!([1,2]);
        let a_index = json!(0);
        let b = json!([3,4]);
        let b_index = json!(1);

        let inputs = vec![a, a_index, b, b_index];

        let (result, _) = _multiply_row(&inputs).expect("_multiply_row() failed");

        let output = result.expect("Could not get the Value from the output");

        assert_eq!(output.pointer("/product").expect("Could not get 'product' output"),
                   &json!(11));
        assert_eq!(output.pointer("/a_b_index").expect("Could not get 'product' output"),
                   &json!([0,1]));
    }
}
