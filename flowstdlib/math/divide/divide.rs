extern crate flow_impl_derive;
#[macro_use]
extern crate serde_json;

use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
pub struct Divide;

impl Divide {
    fn run(&self, inputs: Vec<Vec<Value>>) -> (Option<Value>, bool) {
        let dividend = inputs.get(0).unwrap()[0].as_f64().unwrap();
        let divisor = inputs.get(1).unwrap()[0].as_f64().unwrap();

        let output = json!({"dividend:": dividend, "divisor": divisor, "result": dividend/divisor, "remainder": dividend % divisor});

        (Some(output), true)
    }
}

#[cfg(test)]
mod test {
    use serde_json::Value;

    use super::Divide;

    #[test]
    fn test_divide() {
        let divide = &Divide{};

        // Create input vector
        let dividend = json!(99);
        let divisor = json!(3);
        let inputs: Vec<Vec<Value>> = vec!(vec!(dividend), vec!(divisor));

        divide.run(inputs);
    }
}