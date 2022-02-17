use flowmacro::flow_function;
use serde_json::Value;

#[flow_function]
fn _buffer(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    Ok((Some(inputs[0].clone()), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use serde_json::Value;

    use super::_buffer;

    #[test]
    fn buffer_returns_value() {
        let value: Value = json!(42);

        let buffered_value = _buffer(&[value]).expect("_buffer() failed").0.expect("Could not get the Value from the output");
        assert_eq!(buffered_value, 42, "Did not return the value passed in");
    }

    #[test]
    fn buffer_always_runs_again() {
        let value: Value = json!(42);

        let runs_again = _buffer(&[value]).expect("_buffer() failed").1;
        assert!(runs_again, "Buffer should always be available to run again");
    }
}
