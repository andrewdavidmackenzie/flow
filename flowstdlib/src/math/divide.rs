use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use serde_json::Value as JsonValue;

pub struct Divide;

impl Implementation for Divide {
    fn run(&self, inputs: Vec<Vec<JsonValue>>) -> (Option<JsonValue>, RunAgain) {
        let dividend = inputs.get(0).unwrap()[0].as_f64().unwrap();
        let divisor = inputs.get(1).unwrap()[0].as_f64().unwrap();

        let output = json!({"dividend:": dividend, "divisor": divisor, "result": dividend/divisor, "remainder": dividend % divisor});

        (Some(output), true)
    }
}

#[cfg(test)]
mod test {
    use flowrlib::implementation::Implementation;
    use serde_json::Value as JsonValue;
    use super::Divide;

    #[test]
    fn test_divide() {
        let divide: &Implementation = &Divide{} as &Implementation;

        // Create input vector
        let dividend = json!(99);
        let divisor = json!(3);
        let inputs: Vec<Vec<JsonValue>> = vec!(vec!(dividend), vec!(divisor));

        divide.run(inputs);
    }
}