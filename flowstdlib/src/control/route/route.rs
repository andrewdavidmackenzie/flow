use serde_json::Value;

use flowcore::{RUN_AGAIN, RunAgain};
use flowcore::errors::Result;
use flowmacro::flow_function;

#[flow_function]
fn inner_route(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let data = inputs.first().ok_or("Could not get data")?;
    let control = inputs.get(1).ok_or("Could not get control")?.as_bool().ok_or("Could not get boolean")?;

    let mut output_map = serde_json::Map::new();
    output_map.insert(control.to_string(), data.clone());

    Ok((Some(Value::Object(output_map)), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use flowcore::RUN_AGAIN;

    use super::inner_route;

    #[test]
    fn test_route_true() {
        let inputs = vec![json!(42), json!(true)];
        let (output, run_again) = inner_route(&inputs).expect("_route() failed");
        assert_eq!(run_again, RUN_AGAIN);

        assert!(output.is_some());
        let value = output.expect("Could not get the Value from the output");
        let map = value.as_object().expect("Could not get the object from the output");
        assert_eq!(map.get("true").expect("No 'true' value in map"), &json!(42));
        assert!(!map.contains_key("false"));
    }

    #[test]
    fn test_route_false() {
        let inputs = vec![json!(42), json!(false)];
        let (output, run_again) = inner_route(&inputs).expect("_route() failed");
        assert_eq!(run_again, RUN_AGAIN);

        assert!(output.is_some());
        let value = output.expect("Could not get the Value from the output");
        let map = value.as_object().expect("Could not get the object from the output");
        assert_eq!(
            map.get("false").expect("No 'false' value in map"),
            &json!(42)
        );
        assert!(!map.contains_key("true"));
    }
}
