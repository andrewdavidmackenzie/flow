use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;
use serde_json::{json, Value};

fn count_neighbors(grid: &[u8], width: usize, height: usize, x: usize, y: usize) -> u8 {
    let mut count = 0;
    for dy in [-1i32, 0, 1] {
        for dx in [-1i32, 0, 1] {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = (x as i32 + dx).rem_euclid(width as i32) as usize;
            let ny = (y as i32 + dy).rem_euclid(height as i32) as usize;
            if grid[ny * width + nx] != 0 {
                count += 1;
            }
        }
    }
    count
}

fn next_generation(grid: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut new_grid = vec![0u8; width * height];
    for y in 0..height {
        for x in 0..width {
            let neighbors = count_neighbors(grid, width, height, x, y);
            let alive = grid[y * width + x] != 0;
            new_grid[y * width + x] = match (alive, neighbors) {
                (true, 2) | (true, 3) => 255,
                (false, 3) => 255,
                _ => 0,
            };
        }
    }
    new_grid
}

fn seed_pattern(name: &str, width: usize, height: usize) -> Vec<u8> {
    let mut grid = vec![0u8; width * height];
    let cx = width / 2;
    let cy = height / 2;

    let cells: Vec<(usize, usize)> = match name {
        "blinker" => vec![(cx - 1, cy), (cx, cy), (cx + 1, cy)],
        "glider" => vec![
            (cx, cy - 1),
            (cx + 1, cy),
            (cx - 1, cy + 1),
            (cx, cy + 1),
            (cx + 1, cy + 1),
        ],
        "block" => vec![(cx, cy), (cx + 1, cy), (cx, cy + 1), (cx + 1, cy + 1)],
        "rpentomino" => vec![
            (cx, cy - 1),
            (cx + 1, cy - 1),
            (cx - 1, cy),
            (cx, cy),
            (cx, cy + 1),
        ],
        _ => vec![(cx, cy)],
    };

    for (x, y) in cells {
        if x < width && y < height {
            grid[y * width + x] = 255;
        }
    }
    grid
}

#[flow_function]
fn step(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let grid_input = inputs.first().ok_or("Could not get grid")?;
    let size = inputs
        .get(1)
        .ok_or("Could not get size")?
        .as_array()
        .ok_or("Could not get size array")?;
    let width = size
        .first()
        .ok_or("Could not get width")?
        .as_i64()
        .ok_or("Could not get width as i64")? as usize;
    let height = size
        .get(1)
        .ok_or("Could not get height")?
        .as_i64()
        .ok_or("Could not get height as i64")? as usize;

    let grid: Vec<u8> = if let Some(seed_name) = grid_input.as_str() {
        seed_pattern(seed_name, width, height)
    } else {
        let arr = grid_input
            .as_array()
            .ok_or("Could not get grid as array")?;
        if arr.first().and_then(|v| v.as_array()).is_some() {
            // 2D array: flatten rows
            let mut flat = Vec::new();
            for row in arr {
                if let Some(cells) = row.as_array() {
                    for v in cells {
                        flat.push(v.as_u64().map(|n| n as u8).unwrap_or(0));
                    }
                }
            }
            flat
        } else {
            // Flat array
            arr.iter().map(|v| v.as_u64().map(|n| n as u8).unwrap_or(0)).collect()
        }
    };

    let new_grid = next_generation(&grid, width, height);

    let grid_2d: Vec<Vec<u8>> = new_grid.chunks(width).map(|row| row.to_vec()).collect();
    let result = json!({"grid": grid_2d});

    Ok((Some(result), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn blinker_oscillates() {
        let grid = seed_pattern("blinker", 5, 5);
        let gen1 = next_generation(&grid, 5, 5);
        let gen2 = next_generation(&gen1, 5, 5);
        assert_eq!(grid, gen2);
    }

    #[test]
    fn block_is_stable() {
        let grid = seed_pattern("block", 6, 6);
        let gen1 = next_generation(&grid, 6, 6);
        assert_eq!(grid, gen1);
    }

    #[test]
    fn flow_function_with_seed() {
        let inputs = vec![json!("glider"), json!([16, 16])];
        let (output, run_again) = step(&inputs).expect("step failed");
        assert!(run_again);
        let output = output.expect("no output");
        let grid = output["grid"].as_array().expect("grid not array");
        assert_eq!(grid.len(), 16);
        assert_eq!(grid[0].as_array().expect("row").len(), 16);
    }

    #[test]
    fn flow_function_with_2d_grid() {
        let flat = seed_pattern("blinker", 5, 5);
        let grid_2d: Vec<Vec<u8>> = flat.chunks(5).map(|r| r.to_vec()).collect();
        let inputs = vec![json!(grid_2d), json!([5, 5])];
        let (output, run_again) = step(&inputs).expect("step failed");
        assert!(run_again);
        let output = output.expect("no output");
        let rows = output["grid"].as_array().expect("grid not array");
        assert_eq!(rows.len(), 5);
    }
}
