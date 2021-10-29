use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};
use serde_json::value::Value::Number;
use serde_json::Value;

#[derive(FlowImpl)]
/// Compare two input values and output a map of booleans depending on if the comparison
/// is equal, greater than, greater than or equal, less than or less than or equal.
#[derive(Debug)]
pub struct Compare;

impl Implementation for Compare {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let input_a = &inputs[0];
        let input_b = &inputs[1];

        let mut output_map = serde_json::Map::new();

        match (&input_a, &input_b) {
            (&Number(ref a), &Number(ref b)) => {
                if a.is_i64() && b.is_i64() {
                    output_map.insert("equal".into(), Value::Bool(a.as_i64() == b.as_i64()));
                    output_map.insert("lt".into(), Value::Bool(a.as_i64() < b.as_i64()));
                    output_map.insert("gt".into(), Value::Bool(a.as_i64() > b.as_i64()));
                    output_map.insert("lte".into(), Value::Bool(a.as_i64() <= b.as_i64()));
                    output_map.insert("gte".into(), Value::Bool(a.as_i64() >= b.as_i64()));
                    return (Some(Value::Object(output_map)), RUN_AGAIN);
                } else if a.is_u64() && b.is_u64() {
                    output_map.insert("equal".into(), Value::Bool(a.as_u64() == b.as_u64()));
                    output_map.insert("lt".into(), Value::Bool(a.as_u64() < b.as_u64()));
                    output_map.insert("gt".into(), Value::Bool(a.as_u64() > b.as_u64()));
                    output_map.insert("lte".into(), Value::Bool(a.as_u64() <= b.as_u64()));
                    output_map.insert("gte".into(), Value::Bool(a.as_u64() >= b.as_u64()));
                    return (Some(Value::Object(output_map)), RUN_AGAIN);
                } else {
                    match (a.as_f64(), b.as_f64()) {
                        (Some(l), Some(r)) => {
                            output_map
                                .insert("equal".into(), Value::Bool((l - r).abs() < f64::EPSILON));
                            output_map.insert("lt".into(), Value::Bool(l < r));
                            output_map.insert("gt".into(), Value::Bool(l > r));
                            output_map.insert("lte".into(), Value::Bool(l <= r));
                            output_map.insert("gte".into(), Value::Bool(l >= r));
                            return (Some(Value::Object(output_map)), RUN_AGAIN);
                        }
                        (_, _) => {}
                    }
                }
            }
            (_, _) => {}
        }

        (None, RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use flowcore::Implementation;
    use serde_json::{json, Value};

    use super::Compare;

    fn get_tests() -> Vec<(Value, Value, bool, bool, bool, bool, bool)> {
        vec![
            (
                json!(0),
                json!(0),
                true,  // eq
                false, // lt
                false, // gt
                true,  //lte
                true,  // gte
            ),
            (
                json!(1),
                json!(0),
                false, // eq
                false, // lt
                true,  // gt
                false, //lte
                true,  // gte
            ),
            (
                json!(0),
                json!(1),
                false, // eq
                true,  // lt
                false, // gt
                true,  //lte
                false, // gte
            ),
            // f64
            (
                json!(3.15),
                json!(3.15),
                true,  // eq
                false, // lt
                false, // gt
                true,  //lte
                true,  // gte
            ),
            (
                json!(3.15),
                json!(3.11),
                false, // eq
                false, // lt
                true,  // gt
                false, //lte
                true,  // gte
            ),
            (
                json!(3.11),
                json!(3.15),
                false, // eq
                true,  // lt
                false, // gt
                true,  //lte
                false, // gte
            ),
            (
                json!((i64::MAX as u64 + 10) as u64), // force a u64
                json!((i64::MAX as u64 + 20) as u64), // force a u64
                false,                                // eq
                true,                                 // lt
                false,                                // gt
                true,                                 //lte
                false,                                // gte
            ),
        ]
    }

    fn get_inputs(pair: &(Value, Value, bool, bool, bool, bool, bool)) -> Vec<Value> {
        vec![pair.0.clone(), pair.1.clone()]
    }

    #[test]
    fn positive_tests() {
        let comparer = Compare {};

        for test in &get_tests() {
            let (output, again) = comparer.run(&get_inputs(test));

            assert!(again);

            let outputs = output.unwrap();

            assert_eq!(
                outputs.pointer("/equal").unwrap().as_bool().unwrap(),
                test.2
            );
            assert_eq!(outputs.pointer("/lt").unwrap().as_bool().unwrap(), test.3);
            assert_eq!(outputs.pointer("/gt").unwrap().as_bool().unwrap(), test.4);
            assert_eq!(outputs.pointer("/lte").unwrap().as_bool().unwrap(), test.5);
            assert_eq!(outputs.pointer("/gte").unwrap().as_bool().unwrap(), test.6);
        }
    }

    #[test]
    fn not_numbers() {
        let comparer = Compare {};

        let (output, again) = comparer.run(&[json!("hello"), json!(1.0)]);
        assert!(again);
        assert_eq!(
            None, output,
            "Should not be able to compare different types"
        );
    }
}
