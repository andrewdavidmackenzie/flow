use flowrlib::value::Value;

fn get_message() -> Value {
    Value::new(Some("Hello-World"), 1)
}

pub fn get_values() -> Vec<Value> {
    let mut values = Vec::<Value>::with_capacity(1);

    values.push(get_message());

    values
}