use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;
use serde_json::{json, Value};

#[flow_function]
fn inner_divide(dividend: &Value, divisor: &Value) -> Result<(Option<Value>, RunAgain)> {
    if dividend.is_null() || divisor.is_null() {
        let mut null_map = serde_json::Map::new();
        null_map.insert("result".into(), Value::Null);
        null_map.insert("remainder".into(), Value::Null);
        return Ok((Some(Value::Object(null_map)), RUN_AGAIN));
    }

    let mut output_map = serde_json::Map::new();

    let dividend = dividend
        .as_f64()
        .ok_or("Could not get dividend as number")?;
    let divisor = divisor.as_f64().ok_or("Could not get divisor as number")?;
    output_map.insert("result".into(), json!(dividend / divisor));
    output_map.insert("remainder".into(), json!(dividend % divisor));

    Ok((Some(Value::Object(output_map)), RUN_AGAIN))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::{json, Value};

    use super::inner_divide;

    #[test]
    fn divide_exact() {
        let inputs = vec![json!(99), json!(3)];
        let (output, again) = inner_divide(&inputs[0], &inputs[1]).expect("divide failed");
        assert!(again);
        let out = output.expect("no output");
        assert_eq!(*out.pointer("/result").expect("no result"), json!(33.0));
        assert_eq!(
            *out.pointer("/remainder").expect("no remainder"),
            json!(0.0)
        );
    }

    #[test]
    fn divide_with_remainder() {
        let inputs = vec![json!(100), json!(3)];
        let (output, _) = inner_divide(&inputs[0], &inputs[1]).expect("divide failed");
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
        let inputs = vec![json!(10.5), json!(2.5)];
        let (output, _) = inner_divide(&inputs[0], &inputs[1]).expect("divide failed");
        let out = output.expect("no output");
        assert_eq!(*out.pointer("/result").expect("no result"), json!(4.2));
    }

    #[test]
    fn divide_mixed_int_and_float() {
        let inputs = vec![json!(10), json!(2.5)];
        let (output, _) = inner_divide(&inputs[0], &inputs[1]).expect("divide failed");
        let out = output.expect("no output");
        assert_eq!(*out.pointer("/result").expect("no result"), json!(4.0));
    }

    #[test]
    fn divide_null_propagation() {
        let inputs = vec![json!(null), json!(5)];
        let (output, _) = inner_divide(&inputs[0], &inputs[1]).expect("divide failed");
        let out = output.expect("no output");
        assert_eq!(*out.pointer("/result").expect("no result"), Value::Null);
        assert_eq!(
            *out.pointer("/remainder").expect("no remainder"),
            Value::Null
        );
    }
}
