#[cfg(target_arch = "wasm32")]
extern crate core;
#[cfg(target_arch = "wasm32")]
extern crate flow_impl;
#[cfg(target_arch = "wasm32")]
extern crate flow_impl_derive;
#[cfg(target_arch = "wasm32")]
#[macro_use]
extern crate serde_json;

use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
/// Divide one input by another, producing outputs for the dividend, divisor, result and the remainder
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "divide"
/// source = "lib://flowstdlib/math/divide"
/// ```
///
/// ## Inputs
/// * `dividend` - the number to be divided, of type `Number`
/// * `divisor` - the number to divide by, of type `Number`
///
/// ## Outputs
/// * `dividend` - re output the `dividend` input, of type `Number`
/// * `divisor` - re output the `divisor` input, of type `Number`
/// * `result` - the result of the division, of type `Number`
/// * `remainder` - the remainder of the division, of type `Number`
pub struct Divide;

impl Implementation for Divide {
    fn run(&self, inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let dividend = inputs.get(0).unwrap()[0].as_f64().unwrap();
        let divisor = inputs.get(1).unwrap()[0].as_f64().unwrap();


        let mut output_map = serde_json::Map::new();
        output_map.insert("dividend".into(), Value::Number(serde_json::Number::from_f64(dividend).unwrap()));
        output_map.insert("divisor".into(), Value::Number(serde_json::Number::from_f64(divisor).unwrap()));
        output_map.insert("result".into(), Value::Number(serde_json::Number::from_f64(dividend/divisor).unwrap()));
        output_map.insert("remainder".into(), Value::Number(serde_json::Number::from_f64(dividend % divisor).unwrap()));
        let output = Value::Object(output_map);

        (Some(output), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use flow_impl::Implementation;
    use serde_json::Value;

    use super::Divide;

    #[test]
    fn test_divide() {
        let divide: &dyn Implementation = &Divide{} as &dyn Implementation;

        // Create input vector
        let dividend = Value::Number(serde_json::Number::from(99));
        let divisor = Value::Number(serde_json::Number::from(3));
        let inputs: Vec<Vec<Value>> = vec!(vec!(dividend), vec!(divisor));

        divide.run(inputs);
    }
}