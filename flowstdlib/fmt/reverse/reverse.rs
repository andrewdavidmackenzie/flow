use serde_json::json;
use serde_json::Value;
use serde_json::Value::String as JsonString;

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

#[derive(FlowImpl)]
/// Reverse a String
#[derive(Debug)]
pub struct Reverse;

impl Implementation for Reverse {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let mut value = None;

        let input = &inputs[0];
        if let JsonString(ref s) = input {
            value = Some(json!({
                "reversed" : s.chars().rev().collect::<String>(),
                "original": s
            }));
        }

        (value, RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use flowcore::{Implementation, RUN_AGAIN};

    #[test]
    fn test_reverse() {
        let reverser = &super::Reverse {} as &dyn Implementation;
        let inputs = vec![json!("Hello"), json!(true)];
        let (output, run_again) = reverser.run(&inputs);
        assert_eq!(run_again, RUN_AGAIN);

        assert!(output.is_some());
        let value = output.unwrap();
        let map = value.as_object().unwrap();
        assert_eq!(
            map.get("original").expect("No 'original' value in map"),
            &json!("Hello")
        );
        assert_eq!(
            map.get("reversed").expect("No 'reversed' value in map"),
            &json!("olleH")
        );
    }
}
