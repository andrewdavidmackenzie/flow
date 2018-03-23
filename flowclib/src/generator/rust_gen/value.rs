use model::value::Value;

pub fn to_code(value: &Value) -> String {
    let mut code = format!("Value::new(\"{}\".to_string(), {}, ", value.name, value.id);
    let initial_value = value.value.clone();
    if initial_value.is_none() {
        code.push_str("None");
    } else {
        code.push_str(&format!("Some(json!({})),", initial_value.unwrap()));
    }

    // Add tuples of this value's output routes to runnables and the input it's connected to
    code.push_str(" vec!(");
    for ref route in &value.output_routes {
        code.push_str(&format!("(\"{}\", {},{}),", route.0, route.1, route.2));
    }
    code.push_str(")");

    code.push_str(")");
    code
}

#[cfg(test)]
mod test {
    use serde_json::Value as JsonValue;
    use model::value::Value;
    use model::output::Output;
    use super::to_code;

    #[test]
    fn value_to_code() {
        let value = Value {
            name: "value".to_string(),
            datatype: "String".to_string(),
            value: Some(JsonValue::String("Hello-World".to_string())),
            route: "/flow0/value".to_string(),
            outputs: Some(vec!(Output{name: "".to_string(), datatype: "Json".to_string(), route: "".to_string()})),
            output_routes: vec!(("".to_string(), 1, 0)),
            id: 1,
        };

        let code = to_code(&value);
        assert_eq!(code, "Value::new(\"value\".to_string(), 1, Some(json!(\"Hello-World\")), vec!((\"\", 1,0),))")
    }
}