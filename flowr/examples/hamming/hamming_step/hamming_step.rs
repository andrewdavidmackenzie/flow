use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;
use serde_json::{json, Value};

#[flow_function]
fn step(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let candidates = inputs
        .first()
        .ok_or("Could not get candidates")?
        .as_array()
        .ok_or("Could not get candidates as array")?;

    if candidates.is_empty() {
        return Ok((None, false));
    }

    let min = candidates
        .first()
        .ok_or("Empty candidates")?
        .as_i64()
        .ok_or("Could not get min as i64")?;

    let mut next_candidates: Vec<i64> = candidates
        .iter()
        .skip(1)
        .filter_map(|v| v.as_i64())
        .collect();

    for factor in [2, 3, 5] {
        let new_val = min * factor;
        if !next_candidates.contains(&new_val) {
            next_candidates.push(new_val);
        }
    }
    next_candidates.sort_unstable();
    next_candidates.dedup();

    let result = json!({
        "next": min,
        "candidates": next_candidates
    });
    Ok((Some(result), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn first_step() {
        let (result, _) = step(&[json!([1])]).expect("failed");
        let output = result.expect("no output");
        assert_eq!(output["next"], json!(1));
        assert_eq!(output["candidates"], json!([2, 3, 5]));
    }

    #[test]
    fn second_step() {
        let (result, _) = step(&[json!([2, 3, 5])]).expect("failed");
        let output = result.expect("no output");
        assert_eq!(output["next"], json!(2));
        assert_eq!(output["candidates"], json!([3, 4, 5, 6, 10]));
    }

    #[test]
    fn generates_sequence() {
        let mut candidates = json!([1]);
        let mut sequence = vec![];
        for _ in 0..20 {
            let (result, _) = step(&[candidates]).expect("failed");
            let output = result.expect("no output");
            sequence.push(output["next"].as_i64().unwrap());
            candidates = output["candidates"].clone();
        }
        assert_eq!(
            sequence,
            vec![1, 2, 3, 4, 5, 6, 8, 9, 10, 12, 15, 16, 18, 20, 24, 25, 27, 30, 32, 36]
        );
    }
}
