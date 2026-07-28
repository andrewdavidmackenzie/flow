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
