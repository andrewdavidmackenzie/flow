use serde_json::{json, Value};

use flowcore::{RUN_AGAIN, RunAgain};
use flowcore::errors::Result;
use flowmacro::flow_function;

#[flow_function]
fn inner_sort(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    if inputs.first().ok_or("Could not get first")?.is_null() {
        return Ok((Some(Value::Null), RUN_AGAIN));
    }

    let array_num = inputs.first().ok_or("Could not get array_num")?.as_array().ok_or("Could not get array")?;
    let mut array_of_numbers: Vec<Value> = array_num.clone();
    array_of_numbers.sort_by_key(|a| a.as_i64().unwrap_or(0));

    Ok((Some(json!(array_of_numbers)), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::{json, Value};

    use super::inner_sort;

    #[test]
    fn sort_null() {
        let (result, _) = inner_sort(&[Value::Null]).expect("_sort() failed");

        let output = result.expect("Could not get output value");
        assert_eq!(output, Value::Null);
    }

    #[test]
    fn sort_invalid() {
        assert!(inner_sort(&[json!("Hello World")]).is_err());
    }

    #[test]
    fn sort_one() {
        let (result, _) = inner_sort(&[json!([1])]).expect("_sort() failed");

        let output = result.expect("Could not get output value");
        assert_eq!(output, json!([1]));
    }

    #[test]
    fn sort_array() {
        let (result, _) = inner_sort(&[json!([7, 1, 4, 8, 3, 9])]).expect("_sort() failed");

        let output = result.expect("Could not get output value");
        assert_eq!(output, json!([1, 3, 4, 7, 8, 9]));
    }

    #[test]
    fn sort_array_repeats() {
        let (result, _) = inner_sort(&[json!([7, 1, 8, 4, 8, 3, 1, 9])]).expect("_sort() failed");

        let output = result.expect("Could not get output value");
        assert_eq!(output, json!([1, 1, 3, 4, 7, 8, 8, 9]));
    }
}
