use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;
use serde_json::{json, Value};

/// Extract 3x3 neighborhoods for every cell in a flat grid.
///
/// Takes a flat (or 2D) array of cell values and the grid size [width, height].
/// Outputs an array of 3x3 neighborhoods — one per cell.
/// When connected to a downstream input expecting a single array, the runtime
/// auto-decomposes this into individual neighborhoods for parallel processing.
///
/// The grid wraps at boundaries (toroidal topology).
#[flow_function]
fn extract(grid: &Value, size: &Value) -> Result<(Option<Value>, RunAgain)> {
    let cells = grid.as_array().ok_or("Could not get grid as array")?;

    let size_arr = size.as_array().ok_or("Could not get size as array")?;

    #[allow(clippy::cast_sign_loss)]
    let width = size_arr
        .first()
        .and_then(Value::as_i64)
        .ok_or("Could not get width")? as usize;

    #[allow(clippy::cast_sign_loss)]
    let height = size_arr
        .get(1)
        .and_then(Value::as_i64)
        .ok_or("Could not get height")? as usize;

    let total = width * height;

    // Flatten 2D grid if needed
    let flat: Vec<u8> = if cells.first().and_then(|v| v.as_array()).is_some() {
        cells
            .iter()
            .flat_map(|row| {
                row.as_array()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|v| if v.as_u64().unwrap_or(0) > 0 { 1 } else { 0 })
            })
            .collect()
    } else {
        cells
            .iter()
            .map(|v| if v.as_u64().unwrap_or(0) > 0 { 1 } else { 0 })
            .collect()
    };

    // Build array of [x, y, n0..n8] arrays, one per cell.
    // The (x, y) tag ensures correct grid reconstruction after parallel processing.
    let mut neighborhoods: Vec<Value> = Vec::with_capacity(total);
    for y in 0..height {
        for x in 0..width {
            let mut hood = Vec::with_capacity(11);
            hood.push(json!(x));
            hood.push(json!(y));
            for dy in [-1i32, 0, 1] {
                for dx in [-1i32, 0, 1] {
                    let nx = (x as i32 + dx).rem_euclid(width as i32) as usize;
                    let ny = (y as i32 + dy).rem_euclid(height as i32) as usize;
                    hood.push(json!(*flat.get(ny * width + nx).unwrap_or(&0)));
                }
            }
            neighborhoods.push(Value::Array(hood));
        }
    }

    Ok((Some(Value::Array(neighborhoods)), RUN_AGAIN))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use super::extract;

    #[test]
    fn extract_3x3_grid() {
        // 3x3 grid with center cell alive
        let grid = json!([0, 0, 0, 0, 1, 0, 0, 0, 0]);
        let size = json!([3, 3]);
        let (output, _) = extract(&grid, &size).expect("extract failed");
        let hoods = output.unwrap();
        let hoods = hoods.as_array().unwrap();
        assert_eq!(hoods.len(), 9); // one neighborhood per cell
        // Each neighborhood is an 11-element array: [x, y, n0..n8]
        assert_eq!(hoods[0].as_array().unwrap().len(), 11);
    }

    #[test]
    fn center_cell_neighborhood() {
        // 3x3 grid: center=1, all others=0
        let grid = json!([0, 0, 0, 0, 1, 0, 0, 0, 0]);
        let size = json!([3, 3]);
        let (output, _) = extract(&grid, &size).expect("extract failed");
        let hoods = output.unwrap();
        let center_hood = hoods.as_array().unwrap()[4].as_array().unwrap();
        // Center of neighborhood is at index 6 (offset 2 for x,y + offset 4 for center of 3x3)
        assert_eq!(center_hood[6].as_u64().unwrap(), 1);
    }
}
