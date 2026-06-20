use flowcore::errors::Result;
use flowcore::flow_output;
use flowcore::RunAgain;
use flowmacro::flow_function;
use serde_json::{json, Value};

const MIN_WIDTH: usize = 256;
const HEIGHT: usize = 128;

#[flow_function]
fn inner_time_series(values: &Value) -> Result<(Option<Value>, RunAgain)> {
    let values = values.as_array().ok_or("Could not get values as array")?;
    let num_values = values.len().max(1);
    let bar_width = (MIN_WIDTH / num_values).max(1);
    let width = num_values * bar_width;

    let floats: Vec<f64> = values.iter().map(|v| v.as_f64().unwrap_or(0.0)).collect();

    let min_val = floats.iter().copied().fold(f64::INFINITY, f64::min);
    let max_val = floats.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let range = (max_val - min_val).max(f64::MIN_POSITIVE);

    #[allow(clippy::cast_precision_loss)]
    let height_f = HEIGHT as f64;
    let mut grid: Vec<Vec<u8>> = vec![vec![255; width]; HEIGHT];

    for (i, val) in floats.iter().enumerate().take(num_values) {
        let normalized = (val - min_val) / range;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let bar_height = (normalized * height_f).round().max(0.0) as usize;
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
    use serde_json::json;

    use super::inner_time_series;

    #[test]
    fn renders_time_series() {
        let values = json!([10.0, 20.0, 30.0, 20.0]);
        let (result, _) = inner_time_series(&values).expect("failed");
        let output = result.expect("no output");
        let grid = output.pointer("/grid").expect("no grid");
        let rows = grid.as_array().expect("not array");
        assert_eq!(rows.len(), 128);
        let row_width = rows
            .first()
            .expect("empty")
            .as_array()
            .expect("not array")
            .len();
        assert_eq!(row_width, 256);
    }

    #[test]
    fn min_value_has_no_bar() {
        let values = json!([0.0, 50.0, 100.0]);
        let (result, _) = inner_time_series(&values).expect("failed");
        let output = result.expect("no output");
        let grid = output.pointer("/grid").expect("no grid");
        let rows = grid.as_array().expect("not array");
        let bottom_row = rows.last().expect("empty").as_array().expect("not array");
        assert_eq!(
            bottom_row.first().expect("empty"),
            &json!(255),
            "min value should have no bar"
        );
    }

    #[test]
    fn max_value_has_full_bar() {
        let values = json!([0.0, 50.0, 100.0]);
        let (result, _) = inner_time_series(&values).expect("failed");
        let output = result.expect("no output");
        let grid = output.pointer("/grid").expect("no grid");
        let rows = grid.as_array().expect("not array");
        let top_row = rows.first().expect("empty").as_array().expect("not array");
        let last_bar_x = 2 * (256 / 3);
        assert_eq!(
            top_row.get(last_bar_x).expect("out of bounds"),
            &json!(0),
            "max value should have full-height bar"
        );
    }

    #[test]
    fn single_value() {
        let values = json!([42.0]);
        let (result, _) = inner_time_series(&values).expect("failed");
        let output = result.expect("no output");
        let grid = output.pointer("/grid").expect("no grid");
        let rows = grid.as_array().expect("not array");
        assert_eq!(rows.len(), 128);
    }

    #[test]
    fn constant_values() {
        let values = json!([25.0, 25.0, 25.0, 25.0]);
        let (result, _) = inner_time_series(&values).expect("failed");
        let output = result.expect("no output");
        let grid = output.pointer("/grid").expect("no grid");
        let rows = grid.as_array().expect("not array");
        assert_eq!(rows.len(), 128);
    }
}
