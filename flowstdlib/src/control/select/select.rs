use serde_json::Value;

use flowcore::{RUN_AGAIN, RunAgain};
use flowcore::errors::Result;
use flowmacro::flow_function;

#[flow_function]
fn inner_select(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let i1 = inputs.first().ok_or("Could not get i1")?;
    let i2 = inputs.get(1).ok_or("Could not get i2")?;
    let control = inputs.get(2).ok_or("Could not get control")?.as_bool().ok_or("Could not get boolean")?;

    let mut output_map = serde_json::Map::new();
    if control {
        output_map.insert("select_i1".into(), i1.clone());
        output_map.insert("select_i2".into(), i2.clone());
    } else {
        output_map.insert("select_i1".into(), i2.clone());
        output_map.insert("select_i2".into(), i1.clone());
    }

    Ok((Some(Value::Object(output_map)), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use flowcore::RUN_AGAIN;

    use super::inner_select;

    #[test]
    fn test_select_first() {
        let inputs = vec![json!("A"), json!("B"), json!(true)];
        let (output, run_again) = inner_select(&inputs).expect("_select() failed");
        assert_eq!(run_again, RUN_AGAIN);

        assert!(output.is_some());
        let value = output.expect("Could not get the Value from the output");
        let map = value.as_object().expect("Could not get the object from the output");
        assert_eq!(
            map.get("select_i1").expect("No 'select_i1' value in map"),
            &json!("A")
        );
        assert_eq!(
            map.get("select_i2").expect("No 'select_i2' value in map"),
            &json!("B")
        );
    }

    #[test]
    fn test_select_second() {
        let inputs = vec![json!("A"), json!("B"), json!(false)];
        let (output, run_again) = inner_select(&inputs).expect("_select() failed");
        assert_eq!(run_again, RUN_AGAIN);

        assert!(output.is_some());
        let value = output.expect("Could not get the Value from the output");
        let map = value.as_object().expect("Could not get the object from the output");
        assert_eq!(
            map.get("select_i1").expect("No 'select_i1' value in map"),
            &json!("B")
        );
        assert_eq!(
            map.get("select_i2").expect("No 'select_i2' value in map"),
            &json!("A")
        );
    }
}
