use serde_json::json;
use serde_json::Value;
use serde_json::Value::String as JsonString;

use flowcore::{RUN_AGAIN, RunAgain};
use flowcore::errors::Result;
use flowmacro::flow_function;

#[flow_function]
fn inner_reverse(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let mut value = None;

    let input = inputs.first().ok_or("Could not get input")?;
    if let JsonString(ref s) = input {
        value = Some(json!({"reversed" : s.chars().rev().collect::<String>()}));
    }

    Ok((value, RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use flowcore::RUN_AGAIN;

    use super::inner_reverse;

    #[test]
    fn test_reverse() {
        let inputs = vec![json!("Hello"), json!(true)];
        let (output, run_again) = inner_reverse(&inputs).expect("_reverse() failed");
        assert_eq!(run_again, RUN_AGAIN);

        assert!(output.is_some());
        let value = output.expect("No value was returned in the output");
        let map = value.as_object().expect("Expected a object");
        assert_eq!(
            map.get("reversed").expect("No 'reversed' value in map"),
            &json!("olleH")
        );
    }
}
