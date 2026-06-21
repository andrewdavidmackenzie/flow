use serde_json::{json, Value};

use flowcore::errors::Result;
use flowcore::flow_output;
use flowcore::RunAgain;
use flowmacro::flow_function;

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[flow_function]
fn inner_append(s1: &Value, s2: &Value) -> Result<(Option<Value>, RunAgain)> {
    flow_output!(json!(format!(
        "{}{}",
        value_to_string(s1),
        value_to_string(s2)
    )))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use super::inner_append;

    #[test]
    fn append_one_empty_string() {
        let (result, _) = inner_append(&json!(""), &json!("hello")).expect("failed");
        let output = result.expect("no output");
        assert_eq!(output, json!("hello"));
    }

    #[test]
    fn append_two_empty_strings() {
        let (result, _) = inner_append(&json!(""), &json!("")).expect("failed");
        let output = result.expect("no output");
        assert_eq!(output, json!(""));
    }

    #[test]
    fn append_two_strings() {
        let (result, _) = inner_append(&json!("hello"), &json!(" world")).expect("failed");
        let output = result.expect("no output");
        assert_eq!(output, json!("hello world"));
    }

    #[test]
    fn append_string_and_number() {
        let (result, _) = inner_append(&json!("Min: "), &json!(15.3)).expect("failed");
        let output = result.expect("no output");
        assert_eq!(output, json!("Min: 15.3"));
    }

    #[test]
    fn append_number_and_string() {
        let (result, _) = inner_append(&json!(42), &json!(" items")).expect("failed");
        let output = result.expect("no output");
        assert_eq!(output, json!("42 items"));
    }

    #[test]
    fn append_two_numbers() {
        let (result, _) = inner_append(&json!(3.5), &json!(159)).expect("failed");
        let output = result.expect("no output");
        assert_eq!(output, json!("3.5159"));
    }

    #[test]
    fn append_null_ignored() {
        let (result, _) = inner_append(&json!("hello"), &json!(null)).expect("failed");
        let output = result.expect("no output");
        assert_eq!(output, json!("hello"));
    }
}
