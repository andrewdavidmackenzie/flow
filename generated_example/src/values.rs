use flowrlib::value::Value;

pub fn get_values() -> Vec<Box<Value>> {
    let mut values = Vec::<Box<Value>>::with_capacity(1);

    values.push(Box::new(Value::new(Some("Hello-World"), 1)));

    values
}