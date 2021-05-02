use serde_json::Value;

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

#[derive(FlowImpl)]
/// Route data to one or another based on a boolean control value.
#[derive(Debug)]
pub struct Route;

impl Implementation for Route {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let data = &inputs[0];
        let control = inputs[1].as_bool().unwrap_or(false);

        let mut output_map = serde_json::Map::new();
        output_map.insert(control.to_string(), data.clone());

        (Some(Value::Object(output_map)), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use flowcore::{Implementation, RUN_AGAIN};

    #[test]
    fn test_route_true() {
        let router = &super::Route {} as &dyn Implementation;
        let inputs = vec![json!(42), json!(true)];
        let (output, run_again) = router.run(&inputs);
        assert_eq!(run_again, RUN_AGAIN);

        assert!(output.is_some());
        let value = output.unwrap();
        let map = value.as_object().unwrap();
        assert_eq!(map.get("true").expect("No 'true' value in map"), &json!(42));
        assert!(!map.contains_key("false"));
    }

    #[test]
    fn test_route_false() {
        let router = &super::Route {} as &dyn Implementation;
        let inputs = vec![json!(42), json!(false)];
        let (output, run_again) = router.run(&inputs);
        assert_eq!(run_again, RUN_AGAIN);

        assert!(output.is_some());
        let value = output.unwrap();
        let map = value.as_object().unwrap();
        assert_eq!(
            map.get("false").expect("No 'false' value in map"),
            &json!(42)
        );
        assert!(!map.contains_key("true"));
    }
}
