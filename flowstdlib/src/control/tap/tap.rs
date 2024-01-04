use serde_json::Value;

use flowcore::{RUN_AGAIN, RunAgain};
use flowcore::errors::*;
use flowmacro::flow_function;

#[flow_function]
fn _tap(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let mut value = None;
    let data = inputs.first().ok_or("Could not get data")?;
    let control = inputs.get(1).ok_or("Could not get control")?.as_bool().ok_or("Could not get boolean")?;

    if control {
        value = Some(data.clone());
    }

    Ok((value, RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use flowcore::RUN_AGAIN;

    use super::_tap;

    #[test]
    fn test_tap_go() {
        let inputs = vec![json!("A"), json!(true)];
        let (output, run_again) = _tap(&inputs).expect("_tap() failed");
        assert_eq!(run_again, RUN_AGAIN);

        assert!(output.is_some());
        let value = output.expect("Could not get output value");
        assert_eq!(value, json!("A"));
    }

    #[test]
    fn test_tap_no_go() {
        let inputs = vec![json!("A"), json!(false)];
        let (output, run_again) = _tap(&inputs).expect("_tap() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert!(output.is_none());
    }
}
