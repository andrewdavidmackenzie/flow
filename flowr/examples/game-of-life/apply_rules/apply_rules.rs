use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;
use serde_json::{json, Value};

/// Apply Conway's Game of Life rules to a single 3x3 neighborhood.
///
/// Input: a 9-element array representing the 3x3 neighborhood,
/// where index 4 is the center cell. Values are 0 (dead) or 1 (alive).
///
/// Output: 0 or 255 for the new state of the center cell.
/// - Alive cell with 2 or 3 live neighbors → stays alive (255)
/// - Dead cell with exactly 3 live neighbors → becomes alive (255)
/// - Otherwise → dead (0)
#[flow_function]
fn rules(neighborhood: &Value) -> Result<(Option<Value>, RunAgain)> {
    let hood = neighborhood
        .as_array()
        .ok_or("Could not get neighborhood as array")?;

    if hood.len() != 9 {
        return Ok((Some(json!(0)), RUN_AGAIN));
    }

    let center_alive = hood
        .get(4)
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0;

    // Count live neighbors (all cells except center at index 4)
    let mut neighbors: u8 = 0;
    for (i, cell) in hood.iter().enumerate() {
        if i != 4 && cell.as_u64().unwrap_or(0) > 0 {
            neighbors += 1;
        }
    }

    let new_state = match (center_alive, neighbors) {
        (true, 2) | (true, 3) => 255,
        (false, 3) => 255,
        _ => 0,
    };

    Ok((Some(json!(new_state)), RUN_AGAIN))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use serde_json::json;

    use super::rules;

    #[test]
    fn dead_cell_stays_dead() {
        // Dead center, 0 neighbors
        let hood = json!([0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let (output, _) = rules(&hood).expect("rules failed");
        assert_eq!(output.unwrap().as_u64().unwrap(), 0);
    }

    #[test]
    fn dead_cell_with_3_neighbors_born() {
        // Dead center, 3 live neighbors
        let hood = json!([1, 1, 1, 0, 0, 0, 0, 0, 0]);
        let (output, _) = rules(&hood).expect("rules failed");
        assert_eq!(output.unwrap().as_u64().unwrap(), 255);
    }

    #[test]
    fn alive_cell_with_2_neighbors_survives() {
        let hood = json!([1, 1, 0, 0, 1, 0, 0, 0, 0]);
        let (output, _) = rules(&hood).expect("rules failed");
        assert_eq!(output.unwrap().as_u64().unwrap(), 255);
    }

    #[test]
    fn alive_cell_with_3_neighbors_survives() {
        let hood = json!([1, 1, 1, 0, 1, 0, 0, 0, 0]);
        let (output, _) = rules(&hood).expect("rules failed");
        assert_eq!(output.unwrap().as_u64().unwrap(), 255);
    }

    #[test]
    fn alive_cell_with_1_neighbor_dies() {
        let hood = json!([1, 0, 0, 0, 1, 0, 0, 0, 0]);
        let (output, _) = rules(&hood).expect("rules failed");
        assert_eq!(output.unwrap().as_u64().unwrap(), 0);
    }

    #[test]
    fn alive_cell_with_4_neighbors_dies() {
        let hood = json!([1, 1, 1, 1, 1, 0, 0, 0, 0]);
        let (output, _) = rules(&hood).expect("rules failed");
        assert_eq!(output.unwrap().as_u64().unwrap(), 0);
    }
}
