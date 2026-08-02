use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;
use serde_json::{json, Value};

/// Apply Conway's Game of Life rules to a single 3x3 neighborhood.
///
/// Input: an 11-element array: `[x, y, n0, n1, ..., n8]` where
/// `(x, y)` is the cell position and `n0..n8` are the 3x3 neighborhood
/// values. `n4` (index 6 in the array) is the center cell.
///
/// Output: `[x, y, new_state]` where `new_state` is 0 or 255.
#[flow_function]
fn rules(neighborhood: &Value) -> Result<(Option<Value>, RunAgain)> {
    let hood = neighborhood
        .as_array()
        .ok_or("Could not get neighborhood as array")?;

    if hood.len() != 11 {
        return Ok((Some(json!([0, 0, 0])), RUN_AGAIN));
    }

    let x = hood[0].as_u64().unwrap_or(0);
    let y = hood[1].as_u64().unwrap_or(0);

    // Center cell is at index 6 (offset by 2 for the x,y prefix)
    let center_alive = hood
        .get(6)
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0;

    // Count live neighbors (indices 2..=10 except center at 6)
    let mut neighbors: u8 = 0;
    for (i, cell) in hood.iter().enumerate() {
        if i >= 2 && i != 6 && cell.as_u64().unwrap_or(0) > 0 {
            neighbors += 1;
        }
    }

    let new_state = match (center_alive, neighbors) {
        (true, 2) | (true, 3) => 255,
        (false, 3) => 255,
        _ => 0,
    };

    Ok((Some(json!([x, y, new_state])), RUN_AGAIN))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use super::rules;

    #[test]
    fn dead_cell_stays_dead() {
        // (x=5, y=3), dead center, 0 neighbors
        let hood = json!([5, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let (output, _) = rules(&hood).expect("rules failed");
        let arr = output.unwrap();
        let arr = arr.as_array().unwrap();
        assert_eq!(arr[0].as_u64().unwrap(), 5); // x preserved
        assert_eq!(arr[1].as_u64().unwrap(), 3); // y preserved
        assert_eq!(arr[2].as_u64().unwrap(), 0); // dead
    }

    #[test]
    fn dead_cell_with_3_neighbors_born() {
        // (x=7, y=0), dead center, 3 live neighbors
        let hood = json!([7, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0]);
        let (output, _) = rules(&hood).expect("rules failed");
        let arr = output.unwrap();
        let arr = arr.as_array().unwrap();
        assert_eq!(arr[0].as_u64().unwrap(), 7);
        assert_eq!(arr[2].as_u64().unwrap(), 255);
    }

    #[test]
    fn alive_cell_with_2_neighbors_survives() {
        // (0,0), alive center (index 6), 2 neighbors
        let hood = json!([0, 0, 1, 1, 0, 0, 1, 0, 0, 0, 0]);
        let (output, _) = rules(&hood).expect("rules failed");
        let arr = output.unwrap();
        assert_eq!(arr.as_array().unwrap()[2].as_u64().unwrap(), 255);
    }

    #[test]
    fn alive_cell_with_3_neighbors_survives() {
        let hood = json!([0, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0]);
        let (output, _) = rules(&hood).expect("rules failed");
        let arr = output.unwrap();
        assert_eq!(arr.as_array().unwrap()[2].as_u64().unwrap(), 255);
    }

    #[test]
    fn alive_cell_with_1_neighbor_dies() {
        let hood = json!([0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0]);
        let (output, _) = rules(&hood).expect("rules failed");
        let arr = output.unwrap();
        assert_eq!(arr.as_array().unwrap()[2].as_u64().unwrap(), 0);
    }

    #[test]
    fn alive_cell_with_4_neighbors_dies() {
        let hood = json!([0, 0, 1, 1, 1, 1, 1, 0, 0, 0, 0]);
        let (output, _) = rules(&hood).expect("rules failed");
        let arr = output.unwrap();
        assert_eq!(arr.as_array().unwrap()[2].as_u64().unwrap(), 0);
    }
}
