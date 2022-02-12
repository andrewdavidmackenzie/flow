use flow_macro::flow_function;
use serde_json::json;
use serde_json::Value;

#[flow_function]
fn _count(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let mut output_map = serde_json::Map::new();
    output_map.insert("data".into(), inputs[0].clone());

    let mut count = inputs[1].as_i64().ok_or("Could not get count")?;
    count += 1;
    output_map.insert("count".into(), json!(count));

    let output = Value::Object(output_map);

    Ok((Some(output), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::_count;

    #[test]
    fn count_returns_value() {
        let data = json!(42);
        let count = json!(0);
        let inputs = vec![data, count];

        let (result, _) = _count(&inputs).expect("_count() failed");
        let output = result.expect("Could not get the Value from the output");

        assert_eq!(output.pointer("/data").expect("Could not get the /data from the output"), &json!(42));
        assert_eq!(output.pointer("/count").expect("Could not get the /count from the output"), &json!(1));
    }
}
