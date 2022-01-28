use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RUN_AGAIN, RunAgain};
use serde_json::{json, Value};

#[derive(FlowImpl)]
/// Multiply two matrix rows to a product
#[derive(Debug)]
pub struct MultiplyRow;

impl Implementation for MultiplyRow {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let mut product = 0;
        if let Some(row0) = inputs[0].as_array() {
            if let Some(row1) = inputs[1].as_array() {
                for index in 0..row0.len() {
                    if let Some(row0_entry) = row0[index].as_i64() {
                        if let Some(row1_entry) = row1[index].as_i64() {
                            product += row0_entry * row1_entry;
                        }
                    }
                }
            }
        }
        (Some(json!(product)), RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use flowcore::Implementation;
    use serde_json::json;
    use serde_json::Value;

    #[test]
    fn multiply_row() {
        let row0 = Value::Array(vec![json!(1), json!(2)]);
        let row1 = Value::Array(vec![json!(3), json!(4)]);

        let inputs = vec![row0, row1];

        let multiplier = super::MultiplyRow {};
        let (result, _) = multiplier.run(&inputs);

        let product = result.expect("Could not get the Value from the output");

        assert_eq!(product, json!(11));
    }
}
