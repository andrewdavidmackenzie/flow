use serde_json::{json, Value};

use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;

#[flow_function]
fn inner_reverse(input: &str) -> Result<(Option<Value>, RunAgain)> {
    let reversed = input.chars().rev().collect::<String>();
    Ok((Some(json!({"reversed": reversed})), RUN_AGAIN))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use super::inner_reverse;

    #[test]
    fn test_reverse() {
        let (output, _) = inner_reverse("Hello").expect("_reverse() failed");
        let value = output.expect("No output");
        assert_eq!(
            value.pointer("/reversed").expect("No 'reversed'"),
            &json!("olleH")
        );
    }
}
