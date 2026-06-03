use super::numeric_json;
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
fn inner_add(i1: &Value, i2: &Value) -> Result<(Option<Value>, RunAgain)> {
    if i1.is_null() || i2.is_null() {
        return Ok((Some(Value::Null), RUN_AGAIN));
    }

    let result = if let (Some(a_i), Some(b_i)) = (to_integer(i1), to_integer(i2)) {
        a_i.checked_add(b_i).map(|r| json!(r))
    } else if let (Some(a_u), Some(b_u)) = (i1.as_u64(), i2.as_u64()) {
        a_u.checked_add(b_u).map(|r| json!(r))
    } else if let (Some(a_f), Some(b_f)) = (i1.as_f64(), i2.as_f64()) {
        Some(numeric_json(a_f + b_f))
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

    use super::inner_add;

    #[test]
    fn add_integers() {
        let tests: Vec<(Value, Value, Option<Value>)> = vec![
            (json!(0), json!(0), Some(json!(0))),
            (json!(0), json!(-10), Some(json!(-10))),
            (json!(-10), json!(0), Some(json!(-10))),
            (json!(10), json!(20), Some(json!(30))),
            (json!(-10), json!(-20), Some(json!(-30))),
            (json!(10), json!(-20), Some(json!(-10))),
            (json!(-10), json!(20), Some(json!(10))),
        ];

        for (a, b, expected) in &tests {
            let (output, again) = inner_add(a, b).expect("add failed");
            assert!(again);
            assert_eq!(output, *expected);
        }
    }

    #[test]
    fn add_integer_overflow_returns_none() {
        let (output, _) = inner_add(
            &json!(4_660_046_610_375_530_309_i64),
            &json!(7_540_113_804_746_346_429_i64),
        )
        .expect("add failed");
        assert!(output.is_none());

        let (output, _) = inner_add(
            &json!(-4_660_046_610_375_530_309_i64),
            &json!(-7_540_113_804_746_346_429_i64),
        )
        .expect("add failed");
        assert!(output.is_none());
    }

    #[test]
    fn add_floats() {
        let (output, _) = inner_add(&json!(1.5), &json!(2.5)).expect("add failed");
        assert_eq!(output, Some(json!(4)));
    }

    #[test]
    fn add_dot_zero_floats_produce_integer() {
        let (output, _) = inner_add(&json!(1.0), &json!(2.0)).expect("add failed");
        assert_eq!(output, Some(json!(3)));
    }

    #[test]
    fn add_mixed_int_and_float() {
        let (output, _) = inner_add(&json!(1), &json!(2.5)).expect("add failed");
        assert_eq!(output, Some(json!(3.5)));
    }

    #[test]
    fn add_mixed_float_and_int() {
        let (output, _) = inner_add(&json!(1.5), &json!(2)).expect("add failed");
        assert_eq!(output, Some(json!(3.5)));
    }

    #[test]
    fn add_null_propagation() {
        let (output, _) = inner_add(&json!(null), &json!(5)).expect("add failed");
        assert_eq!(output, Some(Value::Null));

        let (output, _) = inner_add(&json!(5), &json!(null)).expect("add failed");
        assert_eq!(output, Some(Value::Null));
    }
}
