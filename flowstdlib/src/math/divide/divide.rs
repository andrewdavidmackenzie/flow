use serde_json::{json, Value};

use flowcore::errors::Result;
use flowcore::flow_output;
use flowcore::RunAgain;
use flowmacro::flow_function;

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::float_cmp
)]
fn numeric_json(f: f64) -> Value {
    if f.fract() == 0.0 && f.abs() < i64::MAX as f64 {
        let i = f as i64;
        if (i as f64) == f {
            return json!(i);
        }
    }
    json!(f)
}

#[flow_function]
fn inner_divide(dividend: &Value, divisor: &Value) -> Result<(Option<Value>, RunAgain)> {
    if dividend.is_null() || divisor.is_null() {
        return flow_output!("result" => Value::Null, "remainder" => Value::Null);
    }

    let dividend = dividend
        .as_f64()
        .ok_or("Could not get dividend as number")?;
    let divisor = divisor.as_f64().ok_or("Could not get divisor as number")?;

    flow_output!(
        "result" => numeric_json(dividend / divisor),
        "remainder" => numeric_json(dividend % divisor)
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::{json, Value};

    use super::inner_divide;

    #[test]
    fn divide_exact() {
        let (output, again) = inner_divide(&json!(99), &json!(3)).expect("divide failed");
        assert!(again);
        let out = output.expect("no output");
        assert_eq!(*out.pointer("/result").expect("no result"), json!(33));
        assert_eq!(*out.pointer("/remainder").expect("no remainder"), json!(0));
    }

    #[test]
    fn divide_with_remainder() {
        let (output, _) = inner_divide(&json!(100), &json!(3)).expect("divide failed");
        let out = output.expect("no output");
        let result = out
            .pointer("/result")
            .expect("no result")
            .as_f64()
            .expect("not f64");
        assert!((result - 33.333_333_333_333_336).abs() < 1e-10);
    }

    #[test]
    fn divide_floats() {
        let (output, _) = inner_divide(&json!(10.5), &json!(2.5)).expect("divide failed");
        let out = output.expect("no output");
        assert_eq!(*out.pointer("/result").expect("no result"), json!(4.2));
    }

    #[test]
    fn divide_mixed_int_and_float() {
        let (output, _) = inner_divide(&json!(10), &json!(2.5)).expect("divide failed");
        let out = output.expect("no output");
        assert_eq!(*out.pointer("/result").expect("no result"), json!(4));
    }

    #[test]
    fn divide_null_propagation() {
        let (output, _) = inner_divide(&json!(null), &json!(5)).expect("divide failed");
        let out = output.expect("no output");
        assert_eq!(*out.pointer("/result").expect("no result"), Value::Null);
        assert_eq!(
            *out.pointer("/remainder").expect("no remainder"),
            Value::Null
        );
    }
}
