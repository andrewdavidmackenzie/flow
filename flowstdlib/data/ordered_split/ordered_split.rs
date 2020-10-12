use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::{json, Value};

#[derive(FlowImpl)]
/// Split a string into (possibly) its parts, based on a separator.
#[derive(Debug)]
pub struct OrderedSplit;

impl Implementation for OrderedSplit {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        if inputs[0].is_null() {
            (Some(Value::Null), RUN_AGAIN)
        } else {
            let string = inputs[0].as_str().unwrap();
            let separator = inputs[1].as_str().unwrap();
            let parts: Vec<&str> = string.split(separator).collect::<Vec<&str>>();
            (Some(json!(parts)), RUN_AGAIN)
        }
    }
}

#[cfg(test)]
mod test {
    use flow_impl::Implementation;
    use serde_json::json;

    #[test]
    fn simple() {
        let string = json!("the quick brown fox jumped over the lazy dog");
        let separator = json!(" ");

        let splitter = super::OrderedSplit {};
        let (result, _) = splitter.run(&[string, separator]);

        let output = result.unwrap();
        let array = output.as_array().unwrap();

        assert_eq!(array.len(), 9);
    }
}