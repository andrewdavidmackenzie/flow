use serde_json::Value;

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

#[derive(FlowImpl)]
/// Select which data to output, based on a boolean control value.
#[derive(Debug)]
pub struct Select;

impl Implementation for Select {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let i1 = &inputs[0];
        let i2 = &inputs[1];
        let control = &inputs[2].as_bool().unwrap();

        let mut output_map = serde_json::Map::new();
        if *control {
            output_map.insert("select_i1".into(), i1.clone());
            output_map.insert("select_i2".into(), i2.clone());
        } else {
            output_map.insert("select_i1".into(), i2.clone());
            output_map.insert("select_i2".into(), i1.clone());
        }

        (Some(Value::Object(output_map)), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use flowcore::{Implementation, RUN_AGAIN};

    #[test]
    fn test_select_first() {
        let selector = &super::Select {} as &dyn Implementation;
        let inputs = vec![json!("A"), json!("B"), json!(true)];
        let (output, run_again) = selector.run(&inputs);
        assert_eq!(run_again, RUN_AGAIN);

        assert!(output.is_some());
        let value = output.unwrap();
        let map = value.as_object().unwrap();
        assert_eq!(
            map.get("select_i1").expect("No 'select_i1' value in map"),
            &json!("A")
        );
        assert_eq!(
            map.get("select_i2").expect("No 'select_i2' value in map"),
            &json!("B")
        );
    }

    #[test]
    fn test_select_second() {
        let selector = &super::Select {} as &dyn Implementation;
        let inputs = vec![json!("A"), json!("B"), json!(false)];
        let (output, run_again) = selector.run(&inputs);
        assert_eq!(run_again, RUN_AGAIN);

        assert!(output.is_some());
        let value = output.unwrap();
        let map = value.as_object().unwrap();
        assert_eq!(
            map.get("select_i1").expect("No 'select_i1' value in map"),
            &json!("B")
        );
        assert_eq!(
            map.get("select_i2").expect("No 'select_i2' value in map"),
            &json!("A")
        );
    }
}
