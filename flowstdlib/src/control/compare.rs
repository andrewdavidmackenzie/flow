use serde_json::Value as JsonValue;
use flowrlib::implementation::Implementation;
use flowrlib::runlist::RunList;
use flowrlib::runnable::Runnable;

pub struct Compare;

/*
    A compare operator that takes two numbers (for now) and outputs the comparisons between them
*/
impl Implementation for Compare {
    fn run(&self, runnable: &Runnable, mut inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> bool {
        let left = inputs[0].remove(0).as_i64().unwrap();
        let right = inputs[1].remove(0).as_i64().unwrap();

        let output = json!({
                    "equal" : left == right,
                    "lt" : left < right,
                    "gt" : left > right,
                    "lte" : left <= right,
                    "gte" : left >= right,
                });
        run_list.send_output(runnable, output);
        true
    }
}