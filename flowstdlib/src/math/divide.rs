use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use flowrlib::process::Process;
use flowrlib::runlist::RunList;
use serde_json::Value as JsonValue;

pub struct Divide;

impl Implementation for Divide {
    fn run(&self, process: &Process, inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> RunAgain {
        let dividend = inputs.get(0).unwrap()[0].as_f64().unwrap();
        let divisor = inputs.get(1).unwrap()[0].as_f64().unwrap();

        let output = json!({"dividend:": dividend, "divisor": divisor, "result": dividend/divisor, "remainder": dividend % divisor});
        run_list.send_output(process, output);

        true
    }
}

#[cfg(test)]
mod test {
    use flowrlib::process::Process;
    use flowrlib::runlist::RunList;
    use serde_json::Value as JsonValue;

    #[test]
    fn test_divide() {
        // Create input vector
        let dividend = json!(99);
        let divisor = json!(3);
        let inputs: Vec<Vec<JsonValue>> = vec!(vec!(dividend), vec!(divisor));

        let mut run_list = RunList::new();
        let d = &Process::new("d",true, "".to_string(), vec!(1, 1, 1), 0, None, vec!()) as &Process;
        let implementation = d.get_implementation();

        implementation.run(d, inputs, &mut run_list);
    }
}