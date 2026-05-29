use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;
use serde_json::{json, Value};

const WIDTH: usize = 256;
const HEIGHT: usize = 128;

#[flow_function]
fn render(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let bins = inputs
        .first()
        .ok_or("Could not get bins")?
        .as_array()
        .ok_or("Could not get bins as array")?;

    let max_count = bins
        .iter()
        .filter_map(|v| v.as_u64())
        .max()
        .unwrap_or(1)
        .max(1);

    let mut grid: Vec<Vec<u8>> = vec![vec![255; WIDTH]; HEIGHT];

    for (x, bin) in bins.iter().enumerate().take(WIDTH) {
        let count = bin.as_u64().unwrap_or(0);
        let bar_height = (count * HEIGHT as u64 / max_count) as usize;
        for y in 0..bar_height {
            grid[HEIGHT - 1 - y][x] = 0;
        }
    }

    Ok((Some(json!({"grid": grid})), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn renders_histogram() {
        let mut bins = vec![json!(0); 256];
        bins[0] = json!(10);
        bins[128] = json!(5);
        bins[255] = json!(10);
        let (result, _) = render(&[Value::Array(bins)]).expect("failed");
        let output = result.expect("no output");
        let grid = output.pointer("/grid").expect("no grid");
        let rows = grid.as_array().expect("not array");
        assert_eq!(rows.len(), 128);
        assert_eq!(rows[0].as_array().expect("not array").len(), 256);
    }
}
