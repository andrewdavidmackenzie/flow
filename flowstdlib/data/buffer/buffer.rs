use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// Takes a value on it's input and sends the same value on it's output when it can
/// run, effectively buffering it until the downstream processs can accept it.
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "buffer"
/// source = "lib://flowstdlib/data/buffer"
/// ```
///
///
/// ## Input
/// * (default) - the value to buffer
///
/// ## Outputs
/// * the buffered value
#[derive(Debug)]
pub struct Buffer;

impl Implementation for Buffer {
    fn run(&self, inputs: &Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        (Some(inputs[0][0].clone()), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use flow_impl::Implementation;
    use serde_json::Value::Number;

    use super::Buffer;

    #[test]
    fn buffer_returns_value() {
        let value: Vec<Vec<serde_json::Value>> = vec!(vec!(Number(serde_json::Number::from(42))));

        let buffer = Buffer {};
        let buffered_value = buffer.run(&value).0.unwrap();
        assert_eq!(buffered_value, 42, "Did not return the value passed in");
    }

    #[test]
    fn buffer_always_runs_again() {
        let value: Vec<Vec<serde_json::Value>> = vec!(vec!(Number(serde_json::Number::from(42))));

        let buffer = Buffer {};
        let runs_again = buffer.run(&value).1;
        assert_eq!(runs_again, true, "Buffer should always be available to run again");
    }
}