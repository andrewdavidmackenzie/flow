use flowcore::errors::Result;
use flowcore::{flow_output, RunAgain};
use flowmacro::flow_function;
use serde_json::{json, Value};

/// Gather `[x, y, value]` triples into a flat grid array.
///
/// The `partial` array is `[received_count, val_0, val_1, ..., val_N-1]`
/// where `received_count` tracks how many cells have arrived and each
/// `val_i` is the cell value at flat index `i = y * width + x`.
///
/// When `received_count` equals `width * height`, the complete grid
/// (without the count prefix) is emitted on the `grid` output.
#[flow_function]
fn inner_gather(
    cell: &Value,
    partial: &Value,
    size: &Value,
) -> Result<(Option<Value>, RunAgain)> {
    let size_arr = size.as_array().ok_or("size must be an array")?;

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let width = size_arr
        .first()
        .and_then(Value::as_u64)
        .ok_or("Could not get width")? as usize;

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let height = size_arr
        .get(1)
        .and_then(Value::as_u64)
        .ok_or("Could not get height")? as usize;

    let total = width * height;

    // Initialize or restore the partial array: [count, val_0, ..., val_N-1]
    let mut partial_arr = if let Some(arr) = partial.as_array() {
        if arr.len() == total + 1 {
            arr.clone()
        } else {
            let mut v = vec![json!(0)]; // count = 0
            v.extend(std::iter::repeat(json!(0)).take(total));
            v
        }
    } else {
        let mut v = vec![json!(0)];
        v.extend(std::iter::repeat(json!(0)).take(total));
        v
    };

    // Extract [x, y, value] from the cell triple
    if let Some(cell_arr) = cell.as_array() {
        if cell_arr.len() >= 3 {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let x = cell_arr[0].as_u64().unwrap_or(0) as usize;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let y = cell_arr[1].as_u64().unwrap_or(0) as usize;
            let value = &cell_arr[2];
            let index = y * width + x;
            if index < total {
                // +1 offset because partial_arr[0] is the count
                partial_arr[index + 1] = value.clone();
            }
        }
    }

    // Increment received count
    let count = partial_arr[0].as_u64().unwrap_or(0) + 1;
    partial_arr[0] = json!(count);

    if count >= total as u64 {
        // All cells received — emit the grid (without the count prefix)
        let grid: Vec<Value> = partial_arr[1..].to_vec();
        flow_output!(
            "grid" => Value::Array(grid),
            "partial" => Value::Array(vec![]),
            "size" => size.clone(),
        )
    } else {
        // Not yet complete — loop back the partial
        flow_output!(
            "partial" => Value::Array(partial_arr),
            "size" => size.clone(),
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;
    use super::inner_gather;

    #[test]
    fn gather_in_order() {
        let size = json!([2, 2]);
        let mut partial = json!([]);

        // Send 4 cells in order
        let cells = vec![
            json!([0, 0, 255]),
            json!([1, 0, 0]),
            json!([0, 1, 0]),
            json!([1, 1, 255]),
        ];

        for (i, cell) in cells.iter().enumerate() {
            let (result, _) = inner_gather(cell, &partial, &size).expect("gather failed");
            let output = result.expect("No output");
            if i < 3 {
                assert!(output.pointer("/grid").is_none(), "Should not emit grid yet");
                partial = output.pointer("/partial").unwrap().clone();
            } else {
                let grid = output.pointer("/grid").unwrap().as_array().unwrap();
                assert_eq!(grid, &vec![json!(255), json!(0), json!(0), json!(255)]);
            }
        }
    }

    #[test]
    fn gather_out_of_order() {
        let size = json!([2, 2]);
        let mut partial = json!([]);

        // Send cells in reverse order
        let cells = vec![
            json!([1, 1, 100]),
            json!([0, 1, 200]),
            json!([1, 0, 300]),
            json!([0, 0, 400]),
        ];

        for (i, cell) in cells.iter().enumerate() {
            let (result, _) = inner_gather(cell, &partial, &size).expect("gather failed");
            let output = result.expect("No output");
            if i < 3 {
                partial = output.pointer("/partial").unwrap().clone();
            } else {
                let grid = output.pointer("/grid").unwrap().as_array().unwrap();
                // Positions: (0,0)=400, (1,0)=300, (0,1)=200, (1,1)=100
                assert_eq!(grid, &vec![json!(400), json!(300), json!(200), json!(100)]);
            }
        }
    }
}
