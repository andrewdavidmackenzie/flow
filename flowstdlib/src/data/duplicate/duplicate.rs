use serde_json::Value;

use flowcore::{RUN_AGAIN, RunAgain};
use flowcore::errors::Result;
use flowmacro::flow_function;

#[flow_function]
fn inner_duplicate(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let value = inputs.first().ok_or("Could not get value")?;

    let mut output_array = vec![];

    let factor = inputs.get(1).ok_or("Could not get factor")?.as_i64().ok_or("Could not get factor")?;
    for _i in 0..factor {
        output_array.push(value.clone());
    }

    Ok((Some(Value::Array(output_array)), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::inner_duplicate;

    #[test]
    fn duplicate_number() {
        let value = json!(42);
        let factor = json!(2);
        let inputs: Vec<serde_json::Value> = vec![value, factor];

        let (output, _) = inner_duplicate(&inputs).expect("_duplicate() failed");

        assert_eq!(output.expect("Could not get the Value from the output"), json!([42, 42]));
    }

    #[test]
    fn duplicate_row_of_numbers() {
        let value = json!([1, 2, 3]);
        let factor = json!(2);
        let inputs: Vec<serde_json::Value> = vec![value, factor];

        let (output, _) = inner_duplicate(&inputs).expect("_duplicate() failed");

        assert_eq!(output.expect("Could not get the Value from the output"), json!([[1, 2, 3], [1, 2, 3]]));
    }

    #[test]
    fn duplicate_matrix() {
        let value = json!([[1, 2, 3], [4, 5, 6], [7, 8, 9]]);
        let factor = json!(2);
        let inputs: Vec<serde_json::Value> = vec![value, factor];

        let (output, _) = inner_duplicate(&inputs).expect("_duplicate() failed");

        assert_eq!(
            output.expect("Could not get the Value from the output"),
            json!([
                [[1, 2, 3], [4, 5, 6], [7, 8, 9]],
                [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
            ])
        );
    }
}
