extern crate core;
extern crate flow_impl;
extern crate flow_impl_derive;
#[cfg(test)]
#[macro_use]
extern crate serde_json;

use flow_impl::implementation::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;
use serde_json::Value::String as JsonString;


#[derive(FlowImpl)]
pub struct Buffer;

impl Implementation for Buffer {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let buffered_value = Some(inputs.remove(0).remove(0));
        (buffered_value, RUN_AGAIN)
    }
}


#[cfg(test)]
mod test {
    use flow_impl::implementation::Implementation;

    use super::Buffer;

    #[test]
    fn buffer_returns_value() {
        let value = vec!(vec!(json!(42)));

        let buffer = Buffer {};
        let buffered_value = buffer.run(value).0.unwrap();
        assert_eq!(buffered_value, 42, "Did not return the value passed in");
    }

    #[test]
    fn buffer_always_runs_again() {
        let value = vec!(vec!(json!(42)));

        let buffer = Buffer {};
        let runs_again = buffer.run(value).1;
        assert_eq!(runs_again, true, "Buffer should always be available to run again");
    }
}