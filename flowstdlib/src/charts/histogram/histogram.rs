use flowcore::errors::Result;
use flowcore::flow_output;
use flowcore::RunAgain;
use flowmacro::flow_function;
use serde_json::{json, Value};

const MIN_WIDTH: usize = 256;
const HEIGHT: usize = 128;

#[flow_function]
fn inner_histogram(bins: &Value) -> Result<(Option<Value>, RunAgain)> {
    let bins = bins.as_array().ok_or("Could not get bins as array")?;
    let num_bins = bins.len().max(1);
    let bar_width = (MIN_WIDTH / num_bins).max(1);
    let width = num_bins * bar_width;

    let max_count = bins
        .iter()
        .filter_map(Value::as_f64)
        .fold(0.0_f64, f64::max)
        .max(f64::MIN_POSITIVE);

    #[allow(clippy::cast_precision_loss)]
    let height_f = HEIGHT as f64;
    let mut grid: Vec<Vec<u8>> = vec![vec![255; width]; HEIGHT];

    for (i, bin) in bins.iter().enumerate().take(num_bins) {
        let count = bin.as_f64().unwrap_or(0.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let bar_height = ((count / max_count) * height_f).round().max(0.0) as usize;
        let bar_height = bar_height.min(HEIGHT);
        let x_start = i * bar_width;
        for y in 0..bar_height {
            if let Some(row) = grid.get_mut(HEIGHT - 1 - y) {
                for dx in 0..bar_width {
                    if let Some(pixel) = row.get_mut(x_start + dx) {
                        *pixel = 0;
                    }
                }
            }
        }
    }

    flow_output!("grid" => json!(grid))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::{json, Value};

    use super::inner_histogram;

    #[test]
    #[allow(clippy::indexing_slicing)]
    fn renders_histogram() {
        let mut bins = vec![json!(0); 256];
        bins[0] = json!(10);
        bins[128] = json!(5);
        bins[255] = json!(10);
        let bins_value = Value::Array(bins);
        let (result, _) = inner_histogram(&bins_value).expect("failed");
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

    #[test]
    fn renders_small_histogram() {
        let bins = vec![json!(10.0), json!(5.0), json!(8.0), json!(3.0)];
        let bins_value = Value::Array(bins);
        let (result, _) = inner_histogram(&bins_value).expect("failed");
        let output = result.expect("no output");
        let grid = output.pointer("/grid").expect("no grid");
        let rows = grid.as_array().expect("not array");
        // 4 bins scaled up to MIN_WIDTH (256), each bar 64px wide
        assert_eq!(rows.len(), 128);
        assert_eq!(
            rows.first()
                .expect("empty")
                .as_array()
                .expect("not array")
                .len(),
            256
        );
        // First bar (value 10) should be full height — bottom row, first column should be black
        let bottom_row = rows.last().expect("empty").as_array().expect("not array");
        assert_eq!(
            *bottom_row.first().expect("empty"),
            json!(0),
            "first bar should be black at bottom"
        );
    }
}
