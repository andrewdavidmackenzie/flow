use serde_json::Value;

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

#[derive(FlowImpl)]
/// Control the flow of data (flow or disappear it) based on a boolean control value.
#[derive(Debug)]
pub struct Tap;

impl Implementation for Tap {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let mut value = None;
        let data = &inputs[0];
        let control = &inputs[1].as_bool().unwrap();
        if *control {
            value = Some(data.clone());
        }

        (value, RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use flowcore::{Implementation, RUN_AGAIN};

    #[test]
    fn test_tap_go() {
        let tap = &super::Tap {} as &dyn Implementation;
        let inputs = vec![json!("A"), json!(true)];
        let (output, run_again) = tap.run(&inputs);
        assert_eq!(run_again, RUN_AGAIN);

        assert!(output.is_some());
        let value = output.unwrap();
        assert_eq!(value, json!("A"));
    }

    #[test]
    fn test_tap_no_go() {
        let tap = &super::Tap {} as &dyn Implementation;
        let inputs = vec![json!("A"), json!(false)];
        let (output, run_again) = tap.run(&inputs);
        assert_eq!(run_again, RUN_AGAIN);
        assert!(output.is_none());
    }
}
