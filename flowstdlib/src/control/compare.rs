use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use flowrlib::process::Process;
use flowrlib::runlist::RunList;
use serde_json::Value as JsonValue;

pub struct Compare;

/*
    A compare operator that takes two numbers (for now) and outputs the comparisons between them
*/
impl Implementation for Compare {
    fn run(&self, process: &Process, mut inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList)
        -> (Option<JsonValue>, RunAgain) {
        let left = inputs[0].remove(0).as_i64().unwrap();
        let right = inputs[1].remove(0).as_i64().unwrap();

        let output = json!({
                    "equal" : left == right,
                    "lt" : left < right,
                    "gt" : left > right,
                    "lte" : left <= right,
                    "gte" : left >= right,
                });

        (Some(output), true)
    }
}