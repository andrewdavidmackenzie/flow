use serde_json::Value;
use serde_json::Value::Number;

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RUN_AGAIN, RunAgain};

#[derive(FlowImpl)]
/// Calculate the square root of a number
#[derive(Debug)]
pub struct Sqrt;

impl Implementation for Sqrt {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let input = &inputs[0];
        let mut value = None;

        if let Number(ref a) = input {
            if a.is_i64() || a.is_u64() || a.is_f64() {
                value = Some(Value::Number(
                    serde_json::Number::from_f64(a.as_f64().unwrap().sqrt()).unwrap(),
                ));
            }
        };

        (value, RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use flowcore::Implementation;

    use super::Sqrt;

    #[test]
    fn test_81() {
        let rooter = Sqrt {};

        let test_81 = json!(81);
        let test_9 = json!(9.0);
        let (root, again) = rooter.run(&[test_81]);

        assert!(again);
        assert_eq!(test_9, root.unwrap());
    }
}
