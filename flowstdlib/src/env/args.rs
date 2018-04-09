use serde_json::Value as JsonValue;
use flowrlib::implementation::Implementation;
use flowrlib::runnable::Runnable;
use flowrlib::runlist::RunList;
use std::env;

pub struct Args;

impl Implementation for Args {
    fn run(&self, runnable: &Runnable, mut _inputs: Vec<Vec<JsonValue>>, run_list: &mut RunList) -> bool {
        if let Ok(args) = env::var("FLOW_ARGS") {
            env::remove_var("FLOW_ARGS"); // so another invocation later won't use it by mistake
            let flow_args: Vec<&str> = args.split(' ').collect();
            run_list.send_output(runnable, json!(flow_args));
        }

        false
    }
}