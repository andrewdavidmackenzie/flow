use serde_json::{json, Value};

use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;

#[flow_function]
fn inner_ordered_split(string: &str, separator: &str) -> Result<(Option<Value>, RunAgain)> {
    let parts: Vec<&str> = string.split(separator).collect::<Vec<&str>>();
    Ok((Some(json!(parts)), RUN_AGAIN))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use super::inner_ordered_split;

    #[test]
    fn simple() {
        let (result, _) = inner_ordered_split("the quick brown fox jumped over the lazy dog", " ")
            .expect("_ordered_split() failed");

        let output = result.expect("Could not get the Value from the output");
        let array = output
            .as_array()
            .expect("Could not get the Array from the output");

        assert_eq!(array.len(), 9);
    }
}
