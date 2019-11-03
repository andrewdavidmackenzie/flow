use std::env;

use flow_impl::{DONT_RUN_AGAIN, Implementation, RunAgain};
use serde_json::Value;

use ::FLOW_ARGS;

#[derive(Debug)]
/// `Implementation` struct for the `get` function
pub struct Get;

impl Implementation for Get {
    fn run(&self, mut _inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let mut value = None;

        if let Ok(args) = env::var(FLOW_ARGS) {
            env::remove_var(FLOW_ARGS); // so another invocation later won't use it by mistake
            let flow_args: Vec<&str> = args.split(' ').collect();
            value = Some(json!(flow_args));
        }

        (value, DONT_RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use std::env;

    use flow_impl::{DONT_RUN_AGAIN, Implementation};

    use super::FLOW_ARGS;
    use super::Get;

    #[test]
    fn test_arg_passing() {
        env::set_var(FLOW_ARGS, "test");

        let get = Get{};
        let (value, again) = get.run(vec!());

        assert_eq!(json!(["test"]), value.unwrap());
        assert_eq!(DONT_RUN_AGAIN, again);
    }
}