use flowcore::errors::Result;
use flowmacro::flow_function;
use serde_json::json;
use serde_json::Value;
use flowcore::RunAgain;
use flowcore::RUN_AGAIN;

#[flow_function]
fn _count(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let mut count = inputs.get(1).ok_or("Could not get count")?.as_i64().ok_or("Could not get count")?;
    count += 1;

    Ok((Some(json!(count)), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::_count;

    #[test]
    fn count_returns_value() {
        let data = json!(42);
        let previous_count = json!(0);
        let inputs = vec![data, previous_count];

        let (result, _) = _count(&inputs).expect("_count() failed");
        let output = result.expect("Could not get the Value from the output");

        assert_eq!(output.pointer("")
                       .expect("Could not get the /count from the output"), &json!(1));
    }
}
