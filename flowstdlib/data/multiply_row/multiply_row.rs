use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::{json, Value};

#[derive(FlowImpl)]
/// Multiply two matrix rows to a product
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "multiply_row"
/// source = "lib://flowstdlib/data/multiply_row"
/// ```
///
/// ## Input
/// name = "a"
/// type = "Array/Number"
///
/// ## Input
/// name = "b"
/// type = "Array/Number"
///
/// ## Output
/// type = "Number"
#[derive(Debug)]
pub struct MultiplyRow;

impl Implementation for MultiplyRow {
    fn run(&self, inputs: &Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let row0 = inputs[0][0].as_array().unwrap();
        let row1 = inputs[1][0].as_array().unwrap();

        let mut product = 0;
        for index in 0..row0.len() {
            product += row0[index].as_i64().unwrap() * row1[index].as_i64().unwrap();
        }

        (Some(json!(product)), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use flow_impl::Implementation;
    use serde_json::json;
    use serde_json::Value;

    #[test]
    fn multiply_row() {
        let row0 = Value::Array(vec!(json!(1), json!(2)));
        let row1 = Value::Array(vec!(json!(3), json!(4)));

        let inputs = vec!(vec!(row0), vec!(row1));

        let multiplier = super::MultiplyRow {};
        let (result, _) = multiplier.run(&inputs);

        let product = result.unwrap();

        assert_eq!(product, json!(11));
    }
}