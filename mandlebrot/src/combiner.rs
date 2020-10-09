use flowrlib::implementation::Implementation;
use flowrlib::runlist::RunList;
use flowrlib::runnable::Runnable;
use serde_json::Value as JsonValue;
use std::str::FromStr;

pub struct Combiner;

/*
*/
impl Implementation for Combiner {
    fn run(&self, runnable: &Runnable, mut inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> bool {
        let input = inputs.remove(0).remove(0);

        // Somehow, magically one day this will generate a single output formed from all the inputs
//                    run_list.send_output(runnable, output);

        true
    }
}

