use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;
use serde_json::{json, Value};

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::float_cmp
)]
fn to_integer(v: &Value) -> Option<i64> {
    if let Some(i) = v.as_i64() {
        return Some(i);
    }
    if v.as_u64().is_some() {
        return None;
    }
    if let Some(f) = v.as_f64() {
        if f.fract() == 0.0 {
            let i = f as i64;
            if (i as f64) == f {
                return Some(i);
            }
        }
    }
    None
}

#[flow_function]
fn inner_multiply(i1: &Value, i2: &Value) -> Result<(Option<Value>, RunAgain)> {
    if i1.is_null() || i2.is_null() {
        return Ok((Some(Value::Null), RUN_AGAIN));
    }

    let result = if let (Some(a_i), Some(b_i)) = (to_integer(i1), to_integer(i2)) {
        a_i.checked_mul(b_i).map(|r| json!(r))
    } else if let (Some(a_u), Some(b_u)) = (i1.as_u64(), i2.as_u64()) {
        a_u.checked_mul(b_u).map(|r| json!(r))
    } else if let (Some(a_f), Some(b_f)) = (i1.as_f64(), i2.as_f64()) {
        Some(json!(a_f * b_f))
    } else {
        None
    };

    Ok((result, RUN_AGAIN))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;
    use serde_json::Value;

    use super::inner_multiply;

    #[test]
    fn multiply_integers() {
        let tests: Vec<(Value, Value, Value)> = vec![
            (json!(3), json!(3), json!(9)),
            (json!(33), json!(3), json!(99)),
            (json!(0), json!(100), json!(0)),
            (json!(-3), json!(3), json!(-9)),
            (json!(-3), json!(-3), json!(9)),
        ];

        for (a, b, expected) in &tests {
            let (output, again) = inner_multiply(&a.clone(), &b.clone()).expect("multiply failed");
            assert!(again);
            assert_eq!(output, Some(expected.clone()));
        }
    }

    #[test]
    fn multiply_floats() {
        let (output, _) = inner_multiply(&json!(2.5), &json!(4.0)).expect("multiply failed");
        assert_eq!(output, Some(json!(10.0)));
    }

    #[test]
    fn multiply_dot_zero_floats_produce_integer() {
        let (output, _) = inner_multiply(&json!(3.0), &json!(4.0)).expect("multiply failed");
        assert_eq!(output, Some(json!(12)));
    }

    #[test]
    fn multiply_mixed_int_and_float() {
        let (output, _) = inner_multiply(&json!(3), &json!(2.5)).expect("multiply failed");
        assert_eq!(output, Some(json!(7.5)));
    }

    #[test]
    fn multiply_mixed_float_and_int() {
        let (output, _) = inner_multiply(&json!(2.5), &json!(4)).expect("multiply failed");
        assert_eq!(output, Some(json!(10.0)));
    }

    #[test]
    fn multiply_null_propagation() {
        let (output, _) = inner_multiply(&json!(null), &json!(5)).expect("multiply failed");
        assert_eq!(output, Some(Value::Null));

        let (output, _) = inner_multiply(&json!(5), &json!(null)).expect("multiply failed");
        assert_eq!(output, Some(Value::Null));
    }
}
