use flow_macro::flow_function;
use serde_json::{json, Value};

#[flow_function]
fn _append(inputs: &[Value]) -> (Option<Value>, RunAgain) {
    let v1 = inputs[0].clone();
    let v2 = inputs[1].clone();

    if v1.is_string() && v2.is_string() {
        let s1 = v1.as_str().unwrap_or("");
        let s2 = v2.as_str().unwrap_or("");
        (Some(json!(format!("{}{}", s1, s2))), RUN_AGAIN)
    } else {
        (None, RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use super::_append;

    #[test]
    fn append_one_empty_string() {
        let s1 = json!("");
        let s2 = json!("hello");

        let (result, _) = _append(&[s1, s2]);
        let output = result.expect("Could not get the Value from the output");
        assert_eq!(output, json!("hello"));
    }

    #[test]
    fn append_two_empty_strings() {
        let s1 = json!("");
        let s2 = json!("");

        let (result, _) = _append(&[s1, s2]);
        let output = result.expect("Could not get the Value from the output");
        assert_eq!(output, json!(""));
    }

    #[test]
    fn append_two_strings() {
        let s1 = json!("hello");
        let s2 = json!(" world");

        let (result, _) = _append(&[s1, s2]);
        let output = result.expect("Could not get the Value from the output");
        assert_eq!(output, json!("hello world"));
    }

    #[test]
    fn append_one_non_string() {
        let s1 = json!("hello");
        let s2 = json!(42);

        let (result, run_again) = _append(&[s1, s2]);
        assert!(result.is_none());
        assert!(run_again);
    }
}
