use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RUN_AGAIN, RunAgain};
use serde_json::{json, Value};

/// Generate numbers within a Range
#[derive(FlowImpl)]
#[derive(Debug)]
pub struct RangeSplit;

impl Implementation for RangeSplit {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        match inputs[0].as_array() {
            Some(min_and_max) => {
                match (min_and_max[0].as_i64(), min_and_max[1].as_i64()) {
                    (Some(min), Some(max)) => {
                        let mut output_map = serde_json::Map::new();

                        if min == max {
                            // if the two are the same, we are done, output in the sequence
                            output_map.insert("same".into(), json!(min));
                        } else {
                            // split the range_split into two and output for further subdivision
                            let bottom: Vec<i64> = vec!(min, ((max-min)/2) + min);
                            output_map.insert("bottom".into(), json!(bottom));

                            let above_middle = ((max-min)/2) + min +1;
                            if above_middle != max {
                                let top: Vec<i64> = vec!(above_middle, max);
                                output_map.insert("top".into(), json!(top));
                            } else {
                                // if the two are the same, we are done, output in the sequence
                                output_map.insert("same".into(), json!(max));
                            }
                        }
                        (Some(json!(output_map)), RUN_AGAIN)
                    }
                    _ => (None, RUN_AGAIN)
                }
            }
            _ => (None, RUN_AGAIN)
        }
    }
}

#[cfg(test)]
mod test {
    use flowcore::Implementation;
    use serde_json::{json, Value};

    use super::RangeSplit;

    #[test]
    fn test_first_split() {
        let ranger = RangeSplit{};

        let range = vec!(1, 10);
        let (output, again) = ranger.run(&[json!(range)]);

        let result = output.expect("Could not get value from output");
        assert!(again);

        assert_eq!(result.pointer("/bottom").expect("Could not get the /bottom from the output"),
            &json!([1, 5]));
        assert_eq!(result.pointer("/top").expect("Could not get the /top from the output"),
            &json!([6, 10]));
    }

    #[test]
    fn test_entire_range() {
        let min = 1;
        let max = 10;
        let test_range: Vec<i32> = vec!(min, max);
        let test_range_json = json!(test_range);

        // these are outputs of "top" or "bottom" that require feeding into further iterations of Range
        let mut requires_further_splitting: Vec<Value> = vec!(test_range_json);

        // We accumulate the values sent on "sequence" and then compare at the end
        let mut acquired_set: Vec<Value> = vec!();

        let ranger = RangeSplit{};

        while let Some(next) = requires_further_splitting.pop() {
            println!("Splitting: {:?}", next);
            let (output, again) = ranger.run(&[next]);
            assert!(again);
            let result = output.expect("Could not get value from output");

            if let Some(bottom) = result.pointer("/bottom") {
                requires_further_splitting.push(bottom.clone());
            }
            if let Some(top) = result.pointer("/top") {
                requires_further_splitting.push(top.clone());
            }
            if let Some(sequence) = result.pointer("/same") {
                acquired_set.push(sequence.clone());
            }
        }

        assert_eq!(acquired_set.len(), 10);
        assert!(acquired_set.contains(&json!(1)));
        assert!(acquired_set.contains(&json!(2)));
        assert!(acquired_set.contains(&json!(3)));
        assert!(acquired_set.contains(&json!(4)));
        assert!(acquired_set.contains(&json!(5)));
        assert!(acquired_set.contains(&json!(6)));
        assert!(acquired_set.contains(&json!(7)));
        assert!(acquired_set.contains(&json!(8)));
        assert!(acquired_set.contains(&json!(9)));
        assert!(acquired_set.contains(&json!(10)));
    }
}
