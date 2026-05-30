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
fn inner_subtract(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    if inputs.iter().any(Value::is_null) {
        return Ok((Some(Value::Null), RUN_AGAIN));
    }

    let a = inputs.first().ok_or("Could not get i1")?;
    let b = inputs.get(1).ok_or("Could not get i2")?;

    let result = if let (Some(a_i), Some(b_i)) = (to_integer(a), to_integer(b)) {
        a_i.checked_sub(b_i).map(|r| json!(r))
    } else if let (Some(a_u), Some(b_u)) = (a.as_u64(), b.as_u64()) {
        a_u.checked_sub(b_u).map(|r| json!(r))
    } else if let (Some(a_f), Some(b_f)) = (a.as_f64(), b.as_f64()) {
        Some(json!(a_f - b_f))
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

    use super::inner_subtract;

    #[test]
    fn subtract_integers() {
        let tests: Vec<(Value, Value, Option<Value>)> = vec![
            (json!(0), json!(0), Some(json!(0))),
            (json!(0), json!(-10), Some(json!(10))),
            (json!(-10), json!(0), Some(json!(-10))),
            (json!(0), json!(10), Some(json!(-10))),
            (json!(10), json!(0), Some(json!(10))),
            (json!(10), json!(20), Some(json!(-10))),
            (json!(-10), json!(-20), Some(json!(10))),
            (json!(10), json!(-20), Some(json!(30))),
            (json!(-10), json!(20), Some(json!(-30))),
        ];

        for (a, b, expected) in &tests {
            let (output, again) = inner_subtract(&[a.clone(), b.clone()]).expect("subtract failed");
            assert!(again);
            assert_eq!(output, *expected);
        }
    }

    #[test]
    fn subtract_large_u64() {
        let (output, _) =
            inner_subtract(&[json!(i64::MAX as u64 + 10), json!(i64::MAX as u64 + 1)])
                .expect("subtract failed");
        assert_eq!(output, Some(json!(9_u64)));
    }

    #[test]
    fn subtract_floats() {
        let (output, _) = inner_subtract(&[json!(5.5), json!(3.0)]).expect("subtract failed");
        assert_eq!(output, Some(json!(2.5)));
    }

    #[test]
    fn subtract_dot_zero_floats_produce_integer() {
        let (output, _) = inner_subtract(&[json!(5.0), json!(3.0)]).expect("subtract failed");
        assert_eq!(output, Some(json!(2)));
    }

    #[test]
    fn subtract_mixed_int_and_float() {
        let (output, _) = inner_subtract(&[json!(10), json!(2.5)]).expect("subtract failed");
        assert_eq!(output, Some(json!(7.5)));
    }

    #[test]
    fn subtract_mixed_float_and_int() {
        let (output, _) = inner_subtract(&[json!(10.5), json!(3)]).expect("subtract failed");
        assert_eq!(output, Some(json!(7.5)));
    }

    #[test]
    fn subtract_null_propagation() {
        let (output, _) = inner_subtract(&[json!(null), json!(5)]).expect("subtract failed");
        assert_eq!(output, Some(Value::Null));

        let (output, _) = inner_subtract(&[json!(5), json!(null)]).expect("subtract failed");
        assert_eq!(output, Some(Value::Null));
    }
}
