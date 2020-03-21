use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// Take 'N' input values (width='N') from the input stream and gather them into a single output item,
/// which is an array of 'N' items long.
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "compose_array"
/// source = "lib://flowstdlib/data/compose_array"
/// ```
///
/// ## Input
/// * type Number
///
/// ## Outputs
/// * type Array of Number (Array/Number)
#[derive(Debug)]
pub struct ComposeArray;

impl Implementation for ComposeArray {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let output_vec = inputs.remove(0);
        let output = Value::Array(output_vec);

        (Some(output), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use flow_impl::Implementation;
    use serde_json::{Number, Value};

    #[ignore]
    #[test]
    fn remove_1() {
        let array: Vec<Value> = vec!(Value::Array(vec!(Value::Number(Number::from(1)),
                                                       Value::Number(Number::from(2)))));
        let value = vec!(Value::Number(Number::from(1)));

        let composer = super::ComposeArray {};
        let (result, _) = composer.run(vec!(value, array));

        assert_eq!(result.unwrap(), Value::Array(vec!(Value::Number(Number::from(2)))));
    }
}