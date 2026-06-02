use serde_json::{json, Value};

use flowcore::errors::Result;
use flowcore::flow_output;
use flowcore::RunAgain;
use flowmacro::flow_function;

#[flow_function]
fn inner_multiply_row(
    a: &Value,
    a_index: &Value,
    b: &Value,
    b_index: &Value,
) -> Result<(Option<Value>, RunAgain)> {
    let mut product = 0;

    let a = a.as_array().ok_or("Could not get a")?;
    let a_index = a_index.as_u64();
    let b = b.as_array().ok_or("Could not get b")?;
    let b_index = b_index.as_u64();

    for index in 0..a.len() {
        if let Some(row0_entry) = a.get(index).ok_or("Could not get entry")?.as_i64() {
            if let Some(row1_entry) = b.get(index).ok_or("Could not get entry")?.as_i64() {
                product += row0_entry * row1_entry;
            }
        }
    }

    flow_output!(
        "product" => json!(product),
        "a_b_index" => json!([a_index, b_index]),
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use super::inner_multiply_row;

    #[test]
    fn multiply_row() {
        let a = json!([1, 2]);
        let a_index = json!(0);
        let b = json!([3, 4]);
        let b_index = json!(1);

        let (result, _) =
            inner_multiply_row(&a, &a_index, &b, &b_index).expect("_multiply_row() failed");

        let output = result.expect("Could not get the Value from the output");

        assert_eq!(
            output
                .pointer("/product")
                .expect("Could not get 'product' output"),
            &json!(11)
        );
        assert_eq!(
            output
                .pointer("/a_b_index")
                .expect("Could not get 'product' output"),
            &json!([0, 1])
        );
    }
}
