use serde_json::value::Value::Number;
use serde_json::Value;

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

#[derive(FlowImpl)]
/// Compare two input values and output different boolean values depending on if the comparison
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
                    (Some(Value::Object(output_map)), RUN_AGAIN)
                } else if a.is_u64() && b.is_u64() {
                    output_map.insert("equal".into(), Value::Bool(a.as_u64() == b.as_u64()));
                    output_map.insert("lt".into(), Value::Bool(a.as_u64() < b.as_u64()));
                    output_map.insert("gt".into(), Value::Bool(a.as_u64() > b.as_u64()));
                    output_map.insert("lte".into(), Value::Bool(a.as_u64() <= b.as_u64()));
                    output_map.insert("gte".into(), Value::Bool(a.as_u64() >= b.as_u64()));
                    (Some(Value::Object(output_map)), RUN_AGAIN)
                } else if a.is_f64() || b.is_f64() {
                    match (a.as_f64(), b.as_f64()) {
                        (Some(l), Some(r)) => {
                            output_map
                                .insert("equal".into(), Value::Bool((l - r).abs() < f64::EPSILON));
                            output_map.insert("lt".into(), Value::Bool(l < r));
                            output_map.insert("gt".into(), Value::Bool(l > r));
                            output_map.insert("lte".into(), Value::Bool(l <= r));
                            output_map.insert("gte".into(), Value::Bool(l >= r));
                            (Some(Value::Object(output_map)), RUN_AGAIN)
                        }
                        (_, _) => {
                            println!("Could not get as f64");
                            (None, RUN_AGAIN)
                        }
                    }
                } else {
                    println!(
                        "Unsupported input types combination in 'compare': {:?}",
                        inputs
                    );
                    (None, RUN_AGAIN)
                }
            }
            (_, _) => {
                println!("Unsupported input types in 'compare': {:?}", inputs);
                (None, RUN_AGAIN)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use serde_json::{json, Value};

    use flowcore::Implementation;

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
        ]
    }

    fn get_inputs(pair: &(Value, Value, bool, bool, bool, bool, bool)) -> Vec<Value> {
        vec![pair.0.clone(), pair.1.clone()]
    }

    #[test]
    fn tests() {
        let comparer = Compare {};

        for test in &get_tests() {
            let (output, again) = comparer.run(&get_inputs(test));

            assert_eq!(true, again);

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
}
