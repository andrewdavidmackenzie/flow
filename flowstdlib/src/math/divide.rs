use serde_json::Value;

use flow_impl::implementation::{Implementation, RunAgain};

pub struct Divide;

impl Implementation for Divide {
    fn run(&self, inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let dividend = inputs.get(0).unwrap()[0].as_f64().unwrap();
        let divisor = inputs.get(1).unwrap()[0].as_f64().unwrap();

        let output = json!({"dividend:": dividend, "divisor": divisor, "result": dividend/divisor, "remainder": dividend % divisor});

        (Some(output), true)
    }
}

#[cfg(test)]
mod test {
    use serde_json::Value;

    use flow_impl::implementation::Implementation;

    use super::Divide;

    #[test]
    fn test_divide() {
        let divide: &dyn Implementation = &Divide{} as &dyn Implementation;

        // Create input vector
        let dividend = json!(99);
        let divisor = json!(3);
        let inputs: Vec<Vec<Value>> = vec!(vec!(dividend), vec!(divisor));

        divide.run(inputs);
    }
}