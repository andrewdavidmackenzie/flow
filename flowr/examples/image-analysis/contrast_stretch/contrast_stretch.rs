use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;
use serde_json::{json, Value};

#[flow_function]
fn stretch(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let pixels = inputs
        .first()
        .ok_or("Could not get pixels")?
        .as_array()
        .ok_or("Could not get pixels as array")?;
    let min = inputs
        .get(1)
        .ok_or("Could not get min")?
        .as_f64()
        .ok_or("Could not get min as f64")?;
    let max = inputs
        .get(2)
        .ok_or("Could not get max")?
        .as_f64()
        .ok_or("Could not get max as f64")?;
    let width = inputs
        .get(3)
        .ok_or("Could not get width")?
        .as_u64()
        .ok_or("Could not get width as u64")? as usize;

    let range = max - min;
    let stretched: Vec<u8> = if range > 0.0 {
        pixels
            .iter()
            .map(|v| {
                let val = v.as_f64().unwrap_or(0.0);
                ((val - min) * 255.0 / range).clamp(0.0, 255.0) as u8
            })
            .collect()
    } else {
        vec![128; pixels.len()]
    };

    let grid: Vec<Vec<u8>> = stretched.chunks(width).map(|row| row.to_vec()).collect();

    Ok((Some(json!({"grid": grid})), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn stretch_to_2d() {
        let pixels = json!([50, 100, 150, 200]);
        let (result, _) = stretch(&[pixels, json!(50), json!(200), json!(2)]).expect("failed");
        let output = result.expect("no output");
        let grid = output["grid"].as_array().unwrap();
        assert_eq!(grid.len(), 2);
        assert_eq!(grid[0].as_array().unwrap().len(), 2);
        assert_eq!(grid[0][0], json!(0));
        assert_eq!(grid[1][1], json!(255));
    }
}
