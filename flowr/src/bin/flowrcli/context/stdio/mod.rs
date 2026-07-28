use serde_json::Value;

/// the `readline` module to allow a flow to read lines from stdin function
pub mod readline;
/// the `stderr` module to allow a flow to send to the stderr function
pub mod stderr;
/// the `stdin` module to allow a flow to get from the stdin function
pub mod stdin;
/// the `stdout` module to allow a flow to send to the stdout function
pub mod stdout;

/// Convert a JSON `Value` to its string representation for output.
/// Strings are returned without surrounding quotes, other types use their
/// JSON/display representation.
fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        _ => value.to_string(),
    }
}

/// Format a value using a format string. The format string uses `{}` as a
/// placeholder for the value, similar to Rust's `format!` macro.
///
/// - If the format string is empty or `"{}"`, the value is returned as-is.
/// - Otherwise, the first `{}` in the format string is replaced with the
///   string representation of the value.
fn format_output(value: &Value, format: &str) -> String {
    let value_str = value_to_string(value);
    if format.is_empty() || format == "{}" {
        value_str
    } else {
        format.replacen("{}", &value_str, 1)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use super::{format_output, value_to_string};

    #[test]
    fn value_to_string_types() {
        assert_eq!(value_to_string(&json!("hello")), "hello");
        assert_eq!(value_to_string(&json!(42)), "42");
        assert_eq!(value_to_string(&json!(2.5)), "2.5");
        assert_eq!(value_to_string(&json!(true)), "true");
        assert_eq!(value_to_string(&json!(null)), "null");
        assert_eq!(value_to_string(&json!([1, 2])), "[1,2]");
    }

    #[test]
    fn format_empty_passes_through() {
        assert_eq!(format_output(&json!(42), ""), "42");
    }

    #[test]
    fn format_bare_placeholder_passes_through() {
        assert_eq!(format_output(&json!(42), "{}"), "42");
    }

    #[test]
    fn format_with_prefix() {
        assert_eq!(format_output(&json!(42), "Count: {}"), "Count: 42");
    }

    #[test]
    fn format_with_suffix() {
        assert_eq!(format_output(&json!(42), "{} items"), "42 items");
    }

    #[test]
    fn format_with_prefix_and_suffix() {
        assert_eq!(
            format_output(&json!(2.5), "value is approximately {}!"),
            "value is approximately 2.5!"
        );
    }

    #[test]
    fn format_string_value() {
        assert_eq!(
            format_output(&json!("world"), "Hello, {}!"),
            "Hello, world!"
        );
    }

    #[test]
    fn format_no_placeholder() {
        assert_eq!(format_output(&json!(42), "static text"), "static text");
    }

    #[test]
    fn format_multiple_placeholders_replaces_first() {
        assert_eq!(format_output(&json!(1), "{} + {} = 2"), "1 + {} = 2");
    }
}
