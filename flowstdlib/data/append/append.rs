use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};
use serde_json::{json, Value};

#[derive(FlowImpl)]
/// Append two strings
#[derive(Debug)]
pub struct Append;

impl Implementation for Append {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
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
}

#[cfg(test)]
mod test {
    use flowcore::Implementation;
    use serde_json::json;

    #[test]
    fn append_one_empty_string() {
        let s1 = json!("");
        let s2 = json!("hello");

        let appender = super::Append {};
        let (result, _) = appender.run(&[s1, s2]);
        let output = result.unwrap();
        assert_eq!(output, json!("hello"));
    }

    #[test]
    fn append_two_empty_strings() {
        let s1 = json!("");
        let s2 = json!("");

        let appender = super::Append {};
        let (result, _) = appender.run(&[s1, s2]);
        let output = result.unwrap();
        assert_eq!(output, json!(""));
    }

    #[test]
    fn append_two_strings() {
        let s1 = json!("hello");
        let s2 = json!(" world");

        let appender = super::Append {};
        let (result, _) = appender.run(&[s1, s2]);
        let output = result.unwrap();
        assert_eq!(output, json!("hello world"));
    }

    #[test]
    fn append_one_non_string() {
        let s1 = json!("hello");
        let s2 = json!(42);

        let appender = super::Append {};
        let (result, run_again) = appender.run(&[s1, s2]);
        assert!(result.is_none());
        assert!(run_again);
    }
}
