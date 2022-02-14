use flow_macro::flow_function;
use serde_json::Value;

#[flow_function]
fn _remove(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    // Inputs
    let value = &inputs[0];
    let input1 = &inputs[1];
    let mut input_array = input1.clone();

    let output = if let Some(array) = input_array.as_array_mut() {
        array.retain(|val| val != value);
        Value::Array(array.to_vec())
    } else {
        input_array
    };

    Ok((Some(output), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::{json, Value};

    use super::_remove;

    #[test]
    fn remove_1() {
        let array: Value = json!([1, 2]);
        let value = json!(1);

        let (result, _) = _remove(&[value, array]).expect("_remove() failed");

        assert_eq!(result.expect("Could not get the Value from the output"), json!([2]));
    }

    #[test]
    fn remove_repeated_entry() {
        let array: Value = json!([1, 2, 2, 3, 4]);
        let value = json!(2);

        let (result, _) = _remove(&[value, array]).expect("_remove() failed");

        assert_eq!(result.expect("Could not get the Value from the output"), json!([1, 3, 4]));
    }

    #[test]
    fn not_remove_3() {
        let array: Value = json!([1, 2]);
        let value = json!(3);

        let (result, _) = _remove(&[value, array]).expect("_remove() failed");

        assert_eq!(result.expect("Could not get the Value from the output"), json!([1, 2]));
    }

    #[test]
    fn try_to_remove_from_empty_array() {
        let array: Value = json!([]);
        let value = json!(3);

        let (result, _) = _remove(&[value, array]).expect("_remove() failed");

        assert_eq!(result.expect("Could not get the Value from the output"), json!([]));
    }

    #[test]
    fn try_to_remove_non_existent_entry() {
        let array: Value = json!([1, 2, 3, 5, 7, 8, 9]);
        let value = json!(6);

        let (result, _) = _remove(&[value, array]).expect("_remove() failed");

        assert_eq!(result.expect("Could not get the Value from the output"), json!([1, 2, 3, 5, 7, 8, 9]));
    }
}
