use serde_json::{json, Value};

use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;

#[flow_function]
fn inner_append(s1: &str, s2: &str) -> Result<(Option<Value>, RunAgain)> {
    Ok((Some(json!(format!("{s1}{s2}"))), RUN_AGAIN))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use super::inner_append;

    #[test]
    fn append_one_empty_string() {
        let (result, _) = inner_append("", "hello").expect("_append() failed");
        let output = result.expect("Could not get the Value from the output");
        assert_eq!(output, json!("hello"));
    }

    #[test]
    fn append_two_empty_strings() {
        let (result, _) = inner_append("", "").expect("_append() failed");
        let output = result.expect("Could not get the Value from the output");
        assert_eq!(output, json!(""));
    }

    #[test]
    fn append_two_strings() {
        let (result, _) = inner_append("hello", " world").expect("_append() failed");
        let output = result.expect("Could not get the Value from the output");
        assert_eq!(output, json!("hello world"));
    }
}
