use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use flowrlib::process::Process;
use flowrlib::runlist::RunList;
use serde_json;
use serde_json::Value as JsonValue;
use serde_json::Value::Number;

pub struct Subtract;

// TODO implementation of `std::ops::Add` might be missing for `&serde_json::Number`

impl Implementation for Subtract {
    fn run(&self, process: &Process, inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList)
        -> (Option<JsonValue>, RunAgain) {
        let input_a = inputs.get(0).unwrap();
        let input_b = inputs.get(1).unwrap();
        let mut value = None;

        match (&input_a[0], &input_b[0]) {
            (&Number(ref a), &Number(ref b)) => {
                let mut n = JsonValue::Null;
                // TODO mixed signed and unsigned integers
                if a.is_i64() && b.is_i64() {
                    n = JsonValue::Number(serde_json::Number::from(a.as_i64().unwrap() - b.as_i64().unwrap()));
                    run_list.send_output(process, n.clone());
                } else if a.is_u64() && b.is_u64() {
                    n = JsonValue::Number(serde_json::Number::from(a.as_u64().unwrap() - b.as_u64().unwrap()));
                    run_list.send_output(process, n.clone());
                } else if a.is_f64() && b.is_f64() {
                    n = JsonValue::Number(serde_json::Number::from_f64(a.as_f64().unwrap() - b.as_f64().unwrap()).unwrap());
                    run_list.send_output(process, n.clone());
                }

                if n != JsonValue::Null {
                    value = Some(n);
                }
            }
            (_, _) => {}
        }

        (value, true)
    }
}