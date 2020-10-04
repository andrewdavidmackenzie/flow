use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::{json, Value};

#[derive(FlowImpl)]
/// Sort an Array of Numbers
///
/// ## Include using
/// ```toml
/// [[process]]
/// source = "lib://flowstdlib/data/sort"
/// ```
///
/// [[input]]
/// type = "Array/Number"
///
/// [[output]]
/// type = "Array/Number"
#[derive(Debug)]
pub struct Sort;

impl Implementation for Sort {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let array = inputs[0].clone();

        if array.is_null() {
            (Some(Value::Null), RUN_AGAIN)
        } else if array.is_array() {
            let mut array_of_numbers:Vec<Value> = array.as_array().unwrap().clone();
            array_of_numbers.sort_by(|a, b| a.as_i64().unwrap().cmp(&b.as_i64().unwrap()));
            (Some(json!(array_of_numbers)), RUN_AGAIN)
        } else {
            (None, RUN_AGAIN)
        }
    }
}

#[cfg(test)]
mod test {
    use flow_impl::Implementation;
    use serde_json::{json, Value};

    #[test]
    fn sort_null() {
        let sort = super::Sort {};
        let (result, _) = sort.run(&[Value::Null]);

        let output = result.unwrap();
        assert_eq!(output, Value::Null);
    }

    #[test]
    fn sort_one() {
        let sort = super::Sort {};
        let (result, _) = sort.run(&[json!([1])]);

        let output = result.unwrap();
        assert_eq!(output, json!([1]));
    }

    #[test]
    fn sort_array() {
        let sort = super::Sort {};
        let (result, _) = sort.run(&[json!([7, 1, 4, 8, 3, 9])]);

        let output = result.unwrap();
        assert_eq!(output, json!([1, 3, 4, 7, 8, 9]));
    }

    #[test]
    fn sort_array_repeats() {
        let sort = super::Sort {};
        let (result, _) = sort.run(&[json!([7, 1, 8, 4, 8, 3, 1, 9])]);

        let output = result.unwrap();
        assert_eq!(output, json!([1, 1, 3, 4, 7, 8, 8, 9]));
    }
}