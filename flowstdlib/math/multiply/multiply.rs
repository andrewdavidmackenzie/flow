use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// Multiply one input by another
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "multiply"
/// source = "lib://flowstdlib/math/multiply"
/// ```
///
/// ## Inputs
/// * `i1` - one number, of type `Number`
/// * `i2` - the other number, of type `Number`
///
/// ## Outputs
/// * the multiplication of i1 and i2, of type `Number`
#[derive(Debug)]
pub struct Multiply;

impl Implementation for Multiply {
    fn run(&self, inputs: &Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let i1 = inputs[0][0].as_u64().unwrap();
        let i2 = inputs[1][0].as_u64().unwrap();

        let result = i1 * i2;
        let output = Value::Number(serde_json::Number::from(result));

        (Some(output), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use flow_impl::Implementation;
    use serde_json::Value;

    use super::Multiply;

    fn do_multiply(test_data: (u32, u32, u32)) {
        let multiplier: &dyn Implementation = &Multiply{} as &dyn Implementation;

        // Create input vector
        let i1 = Value::Number(serde_json::Number::from(test_data.0));
        let i2 = Value::Number(serde_json::Number::from(test_data.1));
        let inputs: Vec<Vec<Value>> = vec!(vec!(i1), vec!(i2));

        let (output, run_again) = multiplier.run(&inputs);
        assert!(run_again);

        let value = output.unwrap();
        assert_eq!(value, Value::Number(serde_json::Number::from(test_data.2)));
    }

    #[test]
    fn test_divide() {
        let test_set = vec!((3, 3, 9), (33, 3, 99));

        for test in test_set {
            do_multiply(test);
        }
    }
}