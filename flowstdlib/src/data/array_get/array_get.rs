use flowcore::errors::Result;
use flowcore::flow_output;
use flowcore::RunAgain;
use flowmacro::flow_function;
use serde_json::{json, Value};

#[flow_function]
fn inner_array_get(array: &Value, index: &Value) -> Result<(Option<Value>, RunAgain)> {
    let array = array.as_array().ok_or("Could not get array as array")?;
    let index = usize::try_from(index.as_u64().ok_or("Could not get index as u64")?)
        .map_err(|_| "Index too large")?;

    let value = array.get(index).cloned().unwrap_or(Value::Null);

    flow_output!(
        "value" => value,
        "array" => json!(array),
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use super::inner_array_get;

    #[test]
    fn get_first() {
        let (result, _) = inner_array_get(&json!([10, 20, 30]), &json!(0)).expect("failed");
        let output = result.expect("no output");
        assert_eq!(*output.pointer("/value").expect("no /value"), json!(10));
    }

    #[test]
    fn get_middle() {
        let (result, _) = inner_array_get(&json!([10, 20, 30]), &json!(1)).expect("failed");
        let output = result.expect("no output");
        assert_eq!(*output.pointer("/value").expect("no /value"), json!(20));
    }

    #[test]
    fn get_last() {
        let (result, _) = inner_array_get(&json!([10, 20, 30]), &json!(2)).expect("failed");
        let output = result.expect("no output");
        assert_eq!(*output.pointer("/value").expect("no /value"), json!(30));
    }

    #[test]
    fn get_out_of_bounds() {
        let (result, _) = inner_array_get(&json!([10, 20]), &json!(5)).expect("failed");
        let output = result.expect("no output");
        assert_eq!(*output.pointer("/value").expect("no /value"), json!(null));
    }

    #[test]
    fn passes_through_array() {
        let (result, _) = inner_array_get(&json!([10, 20, 30]), &json!(0)).expect("failed");
        let output = result.expect("no output");
        assert_eq!(
            *output.pointer("/array").expect("no /array"),
            json!([10, 20, 30])
        );
    }
}
