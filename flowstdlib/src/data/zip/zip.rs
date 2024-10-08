use serde_json::json;
use serde_json::Value;

use flowcore::{RUN_AGAIN, RunAgain};
use flowcore::errors::Result;
use flowmacro::flow_function;

#[flow_function]
fn inner_zip(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let left = inputs.first().ok_or("Could not get left")?.as_array().ok_or("Could not get left array")?;
    let right = inputs.get(1).ok_or("Could not get right")?.as_array().ok_or("Could not get right array")?;
    let tuples = left.iter().zip(right.iter());
    let tuples_vec: Vec<(&Value, &Value)> = tuples.collect();
    Ok((Some(json!(tuples_vec)), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::{json, Value};

    use super::inner_zip;

    #[test]
    fn zip_empty() {
        let left = Value::Array(vec![]);
        let right = Value::Array(vec![]);

        let inputs = vec![left, right];

        let (result, _) = inner_zip(&inputs).expect("_zip() failed");

        let zipped_array = result.expect("Could not get the value from the output");

        assert_eq!(zipped_array, Value::Array(vec!()));
    }

    #[test]
    fn zip_happy() {
        let left = json!(vec![1, 2]);
        let right = json!(vec![3, 4]);

        let inputs = vec![left, right];

        let (result, _) = inner_zip(&inputs).expect("_zip() failed");

        let zipped_array = result.expect("Could not get the value from the output");

        assert_eq!(zipped_array, json!(vec![(1,3), (2,4)]));
    }

    #[test]
    fn zip_invalid_left() {
        let left = json!(1);
        let right = json!(vec![3, 4]);

        let inputs = vec![left, right];

        assert!(inner_zip(&inputs).is_err());
    }

    #[test]
    fn zip_invalid_right() {
        let left = json!(vec![1, 2]);
        let right = json!(3);

        let inputs = vec![left, right];

        assert!(inner_zip(&inputs).is_err());
    }

    #[test]
    fn zip_unequal() {
        let left = json!(vec![1, 2]);
        let right = json!(vec![3]);

        let inputs = vec![left, right];

        let (result, _) = inner_zip(&inputs).expect("_zip() failed");

        let zipped_array = result.expect("Could not get the value from the output");

        assert_eq!(zipped_array, json!(vec![(1,3)]));
    }
}