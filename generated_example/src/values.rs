use flowrlib::value::Value;

static message: Value = Value {
    initial_value: Some("Hello-World"),
    value: None,
    output_count: 0
};

// TODO make mutable and thread safe list of values
pub static values: [&'static Value; 1] = [&message];