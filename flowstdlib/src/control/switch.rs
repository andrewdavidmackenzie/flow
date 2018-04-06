use serde_json::Value as JsonValue;
use flowrlib::implementation::Implementation;
use flowrlib::runlist::RunList;
use flowrlib::runnable::Runnable;

pub struct Switch;

/*
    A control switch function that outputs the "data" input IF the "control" input is true,
    otherwise it does not produce any output
*/
impl Implementation for Switch {
    fn run(&self, runnable: &Runnable, mut inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> bool {
        let data = inputs[0].remove(0);
        let control = inputs[1].remove(0).as_bool().unwrap();
        if control {
            run_list.send_output(runnable, data);
        }
        true
    }
}