use std::env;

use flow_impl::{DONT_RUN_AGAIN, Implementation, RunAgain};
use serde_json::Value;

use ::FLOW_ARGS_NAME;

#[derive(Debug)]
/// `Implementation` struct for the `get` function
pub struct Get;

impl Implementation for Get {
    fn run(&self, mut _inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let mut value = None;

        if let Ok(args) = env::var(FLOW_ARGS_NAME) {
            env::remove_var(FLOW_ARGS_NAME); // so another invocation later won't use it by mistake
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

    use super::FLOW_ARGS_NAME;
    use super::Get;

    #[test]
    fn test_arg_passing() {
        env::set_var(FLOW_ARGS_NAME, "test");

        let get = Get{};
        let (value, again) = get.run(vec!());

        assert_eq!(json!(["test"]), value.unwrap());
        assert_eq!(DONT_RUN_AGAIN, again);
    }
}