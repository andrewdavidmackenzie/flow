use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;
use serde_json::{json, Value};

#[flow_function]
fn histogram(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let pixels = inputs
        .first()
        .ok_or("Could not get pixels")?
        .as_array()
        .ok_or("Could not get pixels as array")?;

    let mut bins = [0u64; 256];
    for pixel in pixels {
        let val = pixel.as_u64().unwrap_or(0).min(255) as usize;
        bins[val] += 1;
    }

    let total: u64 = bins.iter().sum();
    let min = bins.iter().position(|&c| c > 0).unwrap_or(0);
    let max = bins.iter().rposition(|&c| c > 0).unwrap_or(255);
    let weighted_sum: u64 = bins.iter().enumerate().map(|(i, &c)| i as u64 * c).collect::<Vec<_>>().iter().sum();
    let average = if total > 0 { weighted_sum as f64 / total as f64 } else { 0.0 };

    let mut output_map = serde_json::Map::new();
    output_map.insert("min".into(), json!(min));
    output_map.insert("max".into(), json!(max));
    output_map.insert("average".into(), json!(average));
    output_map.insert("count".into(), json!(total));
    output_map.insert("bins".into(), json!(bins.to_vec()));
    output_map.insert(
        "summary".into(),
        json!(format!("min={min} max={max} avg={average:.1} count={total}")),
    );

    Ok((Some(Value::Object(output_map)), RUN_AGAIN))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn simple_histogram() {
        let pixels = json!([0, 0, 128, 255, 255]);
        let (result, _) = histogram(&[pixels]).expect("failed");
        let output = result.expect("no output");
        assert_eq!(output["min"], json!(0));
        assert_eq!(output["max"], json!(255));
        assert_eq!(output["count"], json!(5));
    }

    #[test]
    fn uniform_image() {
        let pixels: Vec<Value> = (0..256).map(|v| json!(v)).collect();
        let (result, _) = histogram(&[Value::Array(pixels)]).expect("failed");
        let output = result.expect("no output");
        assert_eq!(output["min"], json!(0));
        assert_eq!(output["max"], json!(255));
        assert_eq!(output["count"], json!(256));
        let avg = output["average"].as_f64().unwrap();
        assert!((avg - 127.5).abs() < 0.01);
    }
}
