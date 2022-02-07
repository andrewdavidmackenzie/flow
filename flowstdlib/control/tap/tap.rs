use serde_json::Value;

use flow_macro::flow_function;

#[flow_function]
fn _tap(inputs: &[Value]) -> (Option<Value>, RunAgain) {
    let mut value = None;
    let data = &inputs[0];
    if let Some(control) = &inputs[1].as_bool() {
        if *control {
            value = Some(data.clone());
        }
    }

    (value, RUN_AGAIN)
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use flowcore::{RUN_AGAIN};
    use super::_tap;

    #[test]
    fn test_tap_go() {
        let inputs = vec![json!("A"), json!(true)];
        let (output, run_again) = _tap(&inputs);
        assert_eq!(run_again, RUN_AGAIN);

        assert!(output.is_some());
        let value = output.expect("Could not get output value");
        assert_eq!(value, json!("A"));
    }

    #[test]
    fn test_tap_no_go() {
        let inputs = vec![json!("A"), json!(false)];
        let (output, run_again) = _tap(&inputs);
        assert_eq!(run_again, RUN_AGAIN);
        assert!(output.is_none());
    }
}
