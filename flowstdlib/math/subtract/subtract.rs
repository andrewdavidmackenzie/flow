use serde_json::Value;
use serde_json::Value::Number;

use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;

#[derive(FlowImpl)]
/// Subtract one input from another to produce a new output
#[derive(Debug)]
pub struct Subtract;

impl Implementation for Subtract {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let input_a = &inputs[0];
        let input_b = &inputs[1];
        let mut value : Option<Value> = None;

        match (&input_a, &input_b) {
            (&Number(ref a), &Number(ref b)) => {
                // TODO mixed signed and unsigned integers
                if a.is_i64() && b.is_i64() {
                    let result = a.as_i64().unwrap().checked_sub(b.as_i64().unwrap());
                    if let Some(int) = result {
                        value = Some(Value::Number(serde_json::Number::from(int)));
                    }
                } else if a.is_u64() && b.is_u64() {
                    let result = a.as_u64().unwrap().checked_sub(b.as_u64().unwrap());
                    if let Some(int) = result {
                        value = Some(Value::Number(serde_json::Number::from(int)));
                    }
                } else if a.is_f64() && b.is_f64() {
                    let result = a.as_f64().unwrap() - b.as_f64().unwrap();
                    if let Some(f) = serde_json::Number::from_f64(result) {
                        value = Some(Value::Number(f))
                    }
                }
            }
            (_, _) => {}
        }

        (value, RUN_AGAIN)
    }
}


#[cfg(test)]
mod test {
    use serde_json::Value;
    use serde_json::Value::Number;

    use flow_impl::Implementation;

    use super::Subtract;

    fn get_inputs(pair: &(Value, Value, Option<Value>)) -> Vec<Value> {
        vec!(pair.0.clone(), pair.1.clone())
    }

    // 0 minus 0
    // 0 minus negative
    // negative minus 0
    // 0 minus positive
    // positive minus zero
    // positive minus positive
    // negative minus negative
    // positive minus negative
    // negative minus positive
    // overflow minus
    // overflow minus

    // all of those for:
    // integer + integer
    // float plus float
    // float plus integer
    // integer plus float

    // all of those as numbers and strings
    #[test]
    fn test_suber() {
        let integer_test_set = vec!(
            (Number(serde_json::Number::from(0)), Number(serde_json::Number::from(0)), Some(Number(serde_json::Number::from(0)))),
            (Number(serde_json::Number::from(0)), Number(serde_json::Number::from(-10)), Some(Number(serde_json::Number::from(10)))),
            (Number(serde_json::Number::from(-10)), Number(serde_json::Number::from(0)), Some(Number(serde_json::Number::from(-10)))),
            (Number(serde_json::Number::from(0)), Number(serde_json::Number::from(10)), Some(Number(serde_json::Number::from(-10)))),
            (Number(serde_json::Number::from(10)), Number(serde_json::Number::from(0)), Some(Number(serde_json::Number::from(10)))),
            (Number(serde_json::Number::from(10)), Number(serde_json::Number::from(20)), Some(Number(serde_json::Number::from(-10)))),
            (Number(serde_json::Number::from(-10)), Number(serde_json::Number::from(-20)), Some(Number(serde_json::Number::from(10)))),
            (Number(serde_json::Number::from(10)), Number(serde_json::Number::from(-20)), Some(Number(serde_json::Number::from(30)))),
            (Number(serde_json::Number::from(-10)), Number(serde_json::Number::from(20)), Some(Number(serde_json::Number::from(-30)))),
            (Number(serde_json::Number::from(-4_660_046_610_375_530_309 as i64)), Number(serde_json::Number::from(7_540_113_804_746_346_429 as i64)), None),
            (Number(serde_json::Number::from(4_660_046_610_375_530_309 as i64)), Number(serde_json::Number::from(-7_540_113_804_746_346_429 as i64)), None),
        );

        let suber = Subtract {};

        for test in &integer_test_set {
            let (output, again) = suber.run(&get_inputs(test));

            assert_eq!(true, again);

            match output {
                Some(result) => {
                    assert_eq!(result, test.2.clone().unwrap());
                }
                None => {
                    assert!(test.2.is_none())
                }
            }
        }
    }
}