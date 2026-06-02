use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;
use serde_json::Value;

#[flow_function]
fn inner_array_set(
    array: &Value,
    index: &Value,
    value: &Value,
) -> Result<(Option<Value>, RunAgain)> {
    let mut array = array
        .as_array()
        .ok_or("Could not get array as array")?
        .clone();
    let index = usize::try_from(index.as_u64().ok_or("Could not get index as u64")?)
        .map_err(|_| "Index too large")?;
    let value = value.clone();

    if let Some(element) = array.get_mut(index) {
        *element = value;
    }

    Ok((Some(Value::Array(array)), RUN_AGAIN))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use super::inner_array_set;

    #[test]
    fn set_first() {
        let (result, _) =
            inner_array_set(&json!([10, 20, 30]), &json!(0), &json!(99)).expect("failed");
        assert_eq!(result.expect("no output"), json!([99, 20, 30]));
    }

    #[test]
    fn set_middle() {
        let (result, _) =
            inner_array_set(&json!([10, 20, 30]), &json!(1), &json!(99)).expect("failed");
        assert_eq!(result.expect("no output"), json!([10, 99, 30]));
    }

    #[test]
    fn set_last() {
        let (result, _) =
            inner_array_set(&json!([10, 20, 30]), &json!(2), &json!(99)).expect("failed");
        assert_eq!(result.expect("no output"), json!([10, 20, 99]));
    }

    #[test]
    fn set_out_of_bounds_noop() {
        let (result, _) = inner_array_set(&json!([10, 20]), &json!(5), &json!(99)).expect("failed");
        assert_eq!(result.expect("no output"), json!([10, 20]));
    }
}
