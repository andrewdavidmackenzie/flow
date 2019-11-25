use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;
use serde_json::Value::Number;

#[derive(FlowImpl)]
/// Calculate the square root of a number
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "sqrt"
/// source = "lib://flowstdlib/math/sqrt"
/// ```
///
/// ## Inputs
/// * Of type `Number`
///
/// ## Outputs
/// * Square Root of type `Number`
pub struct Sqrt;

impl Implementation for Sqrt {
    fn run(&self, inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let input = inputs.get(0).unwrap();
        let mut value = None;

        match input[0] {
            Number(ref a) => {
                if a.is_i64() {
                    value = Some(Value::Number(serde_json::Number::from_f64(a.as_f64().unwrap().sqrt()).unwrap()));
                } else if a.is_u64() {
                    value = Some(Value::Number(serde_json::Number::from_f64(a.as_f64().unwrap().sqrt()).unwrap()));
                } else if a.is_f64() {
                    value = Some(Value::Number(serde_json::Number::from_f64(a.as_f64().unwrap().sqrt()).unwrap()));
                }
            }
            _ => {}
        }

        (value, RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use flow_impl::Implementation;

    use super::Sqrt;

    #[test]
    fn test_81() {
        let rooter = Sqrt {};

        let test_81 = vec!(vec!(serde_json::Value::Number(serde_json::Number::from(81))));
        let test_9 = serde_json::Value::Number(serde_json::Number::from_f64(9.0).unwrap());
        let (root, again) = rooter.run(test_81);

        println!("root = {:?}", root);
        assert_eq!(true, again);
        assert_eq!(test_9, root.unwrap());
    }
}