use serde_json::Value;

use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;

#[flow_function]
fn inner_tap(data: &Value, control: bool) -> Result<(Option<Value>, RunAgain)> {
    if control {
        Ok((Some(data.clone()), RUN_AGAIN))
    } else {
        Ok((None, RUN_AGAIN))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use super::inner_tap;

    #[test]
    fn test_tap_go() {
        let (output, _) = inner_tap(&json!("A"), true).expect("failed");
        assert_eq!(output, Some(json!("A")));
    }

    #[test]
    fn test_tap_no_go() {
        let (output, _) = inner_tap(&json!("A"), false).expect("failed");
        assert!(output.is_none());
    }
}
