use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};
use serde_json::Value;

#[derive(FlowImpl)]
/// Takes a value on it's input and sends the same value on it's output when it can
/// run, effectively buffering it until the downstream processs can accept it.
#[derive(Debug)]
pub struct Buffer;

impl Implementation for Buffer {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        (Some(inputs[0].clone()), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use flowcore::Implementation;
    use serde_json::json;
    use serde_json::Value;

    use super::Buffer;

    #[test]
    fn buffer_returns_value() {
        let value: Value = json!(42);

        let buffer = Buffer {};
        let buffered_value = buffer.run(&[value]).0.unwrap();
        assert_eq!(buffered_value, 42, "Did not return the value passed in");
    }

    #[test]
    fn buffer_always_runs_again() {
        let value: Value = json!(42);

        let buffer = Buffer {};
        let runs_again = buffer.run(&[value]).1;
        assert!(runs_again, "Buffer should always be available to run again");
    }
}
