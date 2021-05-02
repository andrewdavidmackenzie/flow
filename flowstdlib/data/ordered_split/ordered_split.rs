use serde_json::{json, Value};

use flow_impl_derive::FlowImpl;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

#[derive(FlowImpl)]
/// Split a string into (possibly) its parts, based on a separator.
#[derive(Debug)]
pub struct OrderedSplit;

impl Implementation for OrderedSplit {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        if inputs[0].is_null() {
            (Some(Value::Null), RUN_AGAIN)
        } else {
            let string = inputs[0].as_str().unwrap_or("");
            let separator = inputs[1].as_str().unwrap_or("");
            let parts: Vec<&str> = string.split(separator).collect::<Vec<&str>>();
            (Some(json!(parts)), RUN_AGAIN)
        }
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use flowcore::Implementation;

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
