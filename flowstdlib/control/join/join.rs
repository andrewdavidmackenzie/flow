use flow_macro::flow_function;
use serde_json::Value;

#[flow_function]
fn _join(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let data = Some(inputs[0].clone());
    // second input of 'control' is not used, it just "controls" the execution of this process
    // via it's availability
    Ok((data, RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use flowcore::RUN_AGAIN;
    use serde_json::json;

    use super::_join;

    #[test]
    fn test_join() {
        let inputs = vec![json!(42), json!("OK")];
        let (output, run_again) = _join(&inputs).expect("_join() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(output.expect("No output value"), json!(42));
    }
}
