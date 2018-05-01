use serde_json::Value as JsonValue;
use flowrlib::implementation::Implementation;
use flowrlib::runnable::Runnable;
use flowrlib::runlist::RunList;

pub struct Divide;

impl Implementation for Divide {
    fn run(&self, runnable: &Runnable, inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> bool {
        let dividend = inputs.get(0).unwrap()[0].as_f64().unwrap();
        let divisor = inputs.get(1).unwrap()[0].as_f64().unwrap();

        let output = json!({"dividend:": dividend, "divisor": divisor, "result": dividend/divisor, "remainder": dividend % divisor});
        run_list.send_output(runnable, output);

        true
    }
}

#[cfg(test)]
mod test {
    use flowrlib::runnable::Runnable;
    use flowrlib::runlist::RunList;
    use flowrlib::function::Function;
    use serde_json::Value as JsonValue;
    use super::Divide;

    #[test]
    fn test_divide() {
        // Create input vector
        let dividend = json!(99);
        let divisor = json!(3);
        let inputs: Vec<Vec<JsonValue>> = vec!(vec!(dividend), vec!(divisor));

        let mut run_list = RunList::new();
        let d = &Function::new("d", 3, vec!(1, 1, 1), 0, Box::new(Divide), None, vec!()) as &Runnable;
        let implementation = d.implementation();

        implementation.run(d, inputs, &mut run_list);
    }
}