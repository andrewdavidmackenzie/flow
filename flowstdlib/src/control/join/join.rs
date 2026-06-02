use serde_json::Value;

use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;

#[flow_function]
fn inner_join(data: &Value, control: &Value) -> Result<(Option<Value>, RunAgain)> {
    let _ = control;
    Ok((Some(data.clone()), RUN_AGAIN))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use super::inner_join;

    #[test]
    fn test_join() {
        let (output, _) = inner_join(&json!(42), &json!("OK")).expect("failed");
        assert_eq!(output, Some(json!(42)));
    }
}
