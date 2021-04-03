use serde_json::{json, Value};

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RUN_AGAIN, RunAgain};

#[derive(FlowImpl)]
/// Multiply two matrix rows to a product
#[derive(Debug)]
pub struct MultiplyRow;

impl Implementation for MultiplyRow {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let row0 = inputs[0].as_array().unwrap();
        let row1 = inputs[1].as_array().unwrap();

        let mut product = 0;
        for index in 0..row0.len() {
            product += row0[index].as_i64().unwrap() * row1[index].as_i64().unwrap();
        }

        (Some(json!(product)), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use serde_json::Value;

    use flowcore::Implementation;

    #[test]
    fn multiply_row() {
        let row0 = Value::Array(vec![json!(1), json!(2)]);
        let row1 = Value::Array(vec![json!(3), json!(4)]);

        let inputs = vec![row0, row1];

        let multiplier = super::MultiplyRow {};
        let (result, _) = multiplier.run(&inputs);

        let product = result.unwrap();

        assert_eq!(product, json!(11));
    }
}
