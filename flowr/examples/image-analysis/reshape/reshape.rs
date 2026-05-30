use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;
use serde_json::{json, Value};

#[flow_function]
fn reshape(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let pixels = inputs.first().ok_or("Could not get pixels")?.as_array().ok_or("not array")?;
    let width = usize::try_from(inputs.get(1).ok_or("no width")?.as_u64().ok_or("not u64")?).map_err(|_| "too large")?;
    let grid: Vec<Vec<u8>> = pixels.chunks(width).map(|row| row.iter().map(|v| v.as_f64().unwrap_or(0.0).clamp(0.0, 255.0) as u8).collect()).collect();
    Ok((Some(json!({"grid": grid})), RUN_AGAIN))
}
