use serde_json::Value;

use flowcore::errors::Result;
use flowcore::flow_output;
use flowcore::RunAgain;
use flowmacro::flow_function;

#[flow_function]
fn inner_select(i1: &Value, i2: &Value, control: bool) -> Result<(Option<Value>, RunAgain)> {
    if control {
        flow_output!(
            "select_i1" => i1.clone(),
            "select_i2" => i2.clone(),
        )
    } else {
        flow_output!(
            "select_i1" => i2.clone(),
            "select_i2" => i1.clone(),
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use flowcore::RUN_AGAIN;

    use super::inner_select;

    #[test]
    fn test_select_first() {
        let (output, run_again) =
            inner_select(&json!("A"), &json!("B"), true).expect("_select() failed");
        assert_eq!(run_again, RUN_AGAIN);

        assert!(output.is_some());
        let value = output.expect("Could not get the Value from the output");
        let map = value
            .as_object()
            .expect("Could not get the object from the output");
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
        let (output, run_again) =
            inner_select(&json!("A"), &json!("B"), false).expect("_select() failed");
        assert_eq!(run_again, RUN_AGAIN);

        assert!(output.is_some());
        let value = output.expect("Could not get the Value from the output");
        let map = value
            .as_object()
            .expect("Could not get the object from the output");
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
