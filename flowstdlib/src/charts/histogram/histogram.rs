use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;
use serde_json::{json, Value};

const WIDTH: usize = 256;
const HEIGHT: usize = 128;

#[flow_function]
fn inner_histogram(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let bins = inputs
        .first()
        .ok_or("Could not get bins")?
        .as_array()
        .ok_or("Could not get bins as array")?;

    let max_count = bins
        .iter()
        .filter_map(Value::as_u64)
        .max()
        .unwrap_or(1)
        .max(1);

    let mut grid: Vec<Vec<u8>> = vec![vec![255; WIDTH]; HEIGHT];

    for (x, bin) in bins.iter().enumerate().take(WIDTH) {
        let count = bin.as_u64().unwrap_or(0);
        let bar_height = usize::try_from(count * HEIGHT as u64 / max_count)
            .unwrap_or(0)
            .min(HEIGHT);
        for y in 0..bar_height {
            if let Some(row) = grid.get_mut(HEIGHT - 1 - y) {
                if let Some(pixel) = row.get_mut(x) {
                    *pixel = 0;
                }
            }
        }
    }

    Ok((Some(json!({"grid": grid})), RUN_AGAIN))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::{json, Value};

    use super::inner_histogram;

    #[test]
    fn renders_histogram() {
        let mut bins = vec![json!(0); 256];
        bins[0] = json!(10);
        bins[128] = json!(5);
        bins[255] = json!(10);
        let (result, _) = inner_histogram(&[Value::Array(bins)]).expect("failed");
        let output = result.expect("no output");
        let grid = output.pointer("/grid").expect("no grid");
        let rows = grid.as_array().expect("not array");
        assert_eq!(rows.len(), 128);
        assert_eq!(
            rows.first()
                .expect("empty")
                .as_array()
                .expect("not array")
                .len(),
            256
        );
    }
}
