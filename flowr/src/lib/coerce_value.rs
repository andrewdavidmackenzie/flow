use serde_json::Value;

use flowcore::errors::Result;

/// Coerce a string value using heuristic rules for generic inputs
pub(crate) fn coerce_generic(raw: &str) -> Value {
    let trimmed = raw.trim();

    if trimmed == "null" || trimmed == "Null" {
        return Value::Null;
    }

    if trimmed == "true" {
        return Value::Bool(true);
    }
    if trimmed == "false" {
        return Value::Bool(false);
    }

    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        return Value::String(trimmed[1..trimmed.len() - 1].to_string());
    }

    if let Ok(n) = trimmed.parse::<i64>() {
        return Value::Number(n.into());
    }
    if let Ok(n) = trimmed.parse::<f64>() {
        if let Some(num) = serde_json::Number::from_f64(n) {
            return Value::Number(num);
        }
    }

    if (trimmed.starts_with('[') && trimmed.ends_with(']'))
        || (trimmed.starts_with('{') && trimmed.ends_with('}'))
    {
        if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
            return val;
        }
    }

    Value::String(trimmed.to_string())
}

/// Coerce a string to a Value, validating against an expected JSON type keyword.
/// Returns an error with a descriptive message if coercion fails.
pub(crate) fn coerce_typed(raw: &str, expected_type: &str, input_name: &str) -> Result<Value> {
    let value = coerce_generic(raw);
    let ok = match expected_type {
        "Number" => value.is_number(),
        "String" => value.is_string(),
        "Bool" => value.is_boolean(),
        "Array" => value.is_array(),
        "Object" => value.is_object(),
        "Null" => value.is_null(),
        _ => true,
    };
    if ok {
        Ok(value)
    } else {
        Err(format!("Cannot coerce '{raw}' to {expected_type} for input '{input_name}'").into())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod test {
    use super::*;
    use serde_json::json;

    #[test]
    fn coerce_bool_true() {
        assert_eq!(coerce_generic("true"), json!(true));
    }

    #[test]
    fn coerce_bool_false() {
        assert_eq!(coerce_generic("false"), json!(false));
    }

    #[test]
    fn coerce_null() {
        assert_eq!(coerce_generic("null"), Value::Null);
        assert_eq!(coerce_generic("Null"), Value::Null);
    }

    #[test]
    fn coerce_integer() {
        assert_eq!(coerce_generic("42"), json!(42));
        assert_eq!(coerce_generic("-7"), json!(-7));
    }

    #[test]
    fn coerce_float() {
        assert_eq!(coerce_generic("1.5"), json!(1.5));
    }

    #[test]
    fn coerce_quoted_string() {
        assert_eq!(coerce_generic("\"hello\""), json!("hello"));
        assert_eq!(coerce_generic("\"42\""), json!("42"));
    }

    #[test]
    fn coerce_unquoted_string() {
        assert_eq!(coerce_generic("hello"), json!("hello"));
    }

    #[test]
    fn coerce_array() {
        assert_eq!(coerce_generic("[1,2,3]"), json!([1, 2, 3]));
        assert_eq!(coerce_generic("[]"), json!([]));
    }

    #[test]
    fn coerce_object() {
        assert_eq!(coerce_generic("{\"a\":1}"), json!({"a": 1}));
    }

    #[test]
    fn coerce_typed_number_ok() {
        assert!(coerce_typed("42", "Number", "count").is_ok());
    }

    #[test]
    fn coerce_typed_number_fail() {
        let result = coerce_typed("hello", "Number", "count");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Cannot coerce"));
        assert!(msg.contains("Number"));
        assert!(msg.contains("count"));
    }

    #[test]
    fn coerce_typed_string_ok() {
        assert!(coerce_typed("hello", "String", "name").is_ok());
    }

    #[test]
    fn coerce_typed_unknown_type_passes() {
        assert!(coerce_typed("anything", "CustomType", "x").is_ok());
    }
}
