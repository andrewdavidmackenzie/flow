use serde_json::Value;

use flowcore::{RUN_AGAIN, RunAgain};
use flowcore::errors::*;
use flowmacro::flow_function;

#[flow_function]
fn _join(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let input = inputs.first().ok_or("Could not get input")?;
    let data = Some(input.clone());
    // second input of 'control' is not used, it just "controls" the execution of this process
    // via it's availability
    Ok((data, RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use flowcore::RUN_AGAIN;

    use super::_join;

    #[test]
    fn test_join() {
        let inputs = vec![json!(42), json!("OK")];
        let (output, run_again) = _join(&inputs).expect("_join() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(output.expect("No output value"), json!(42));
    }
}
