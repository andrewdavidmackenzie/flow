use flowcore::errors::Result;
use flowcore::{RUN_AGAIN, RunAgain};
use flowmacro::flow_function;
use serde_json::{json, Value};

const MAX_WINDOW: usize = 24;

#[flow_function]
fn sliding_window(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let value: f64 =
        serde_json::from_value(inputs.get(0).ok_or("missing input: value")?.clone())?;
    let mut window: Vec<f64> =
        serde_json::from_value(inputs.get(1).ok_or("missing input: window")?.clone())?;

    window.push(value);
    if window.len() > MAX_WINDOW {
        window.remove(0);
    }

    Ok((Some(json!({"window": window})), RUN_AGAIN))
}
