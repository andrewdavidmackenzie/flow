use serde_json::Value;

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

#[derive(FlowImpl)]
/// Control the flow of a piece of data by waiting for a second value to be available
#[derive(Debug)]
pub struct Join;

impl Implementation for Join {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let data = Some(inputs[0].clone());
        // second input of 'control' is not used, it just "controls" the execution of this process
        // via it's availability
        (data, RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use flowcore::{Implementation, RUN_AGAIN};

    #[test]
    fn test_join() {
        let joiner = &super::Join {} as &dyn Implementation;
        let inputs = vec![json!(42), json!("OK")];
        let (output, run_again) = joiner.run(&inputs);
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(output.expect("No output value"), json!(42));
    }
}
