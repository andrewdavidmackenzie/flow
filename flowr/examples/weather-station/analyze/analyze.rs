use flowcore::errors::Result;
use flowcore::flow_output;
use flowcore::RunAgain;
use flowmacro::flow_function;
use serde_json::{json, Value};

#[flow_function]
fn analyze(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let window: Vec<f64> =
        serde_json::from_value(inputs.get(0).ok_or("missing input: window")?.clone())?;

    if window.is_empty() {
        return Err("window must contain at least one reading".into());
    }

    let n = window.len();
    let min_temp = window.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_temp = window.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mean_temp = window.iter().sum::<f64>() / n as f64;
    let latest = window[n - 1];

    let mut anomalies = Vec::new();
    for i in 1..n {
        let delta = window[i] - window[i - 1];
        if delta.abs() > 3.0 {
            let direction = if delta > 0.0 { "spike" } else { "drop" };
            anomalies.push(format!(
                "{:.1}\u{00b0}C {} ({:+.1}\u{00b0}C)",
                window[i], direction, delta
            ));
        }
    }

    let mut report = String::new();
    report.push_str(&format!(
        "=== Weather Station ({} readings in window) ===\n",
        n
    ));
    report.push_str(&format!(
        "Latest: {:.1}\u{00b0}C | Min: {:.1}\u{00b0}C | Max: {:.1}\u{00b0}C | Mean: {:.1}\u{00b0}C\n",
        latest, min_temp, max_temp, mean_temp
    ));

    if !anomalies.is_empty() {
        report.push_str(&format!(
            "  ALERT: {} anomalies: {}\n",
            anomalies.len(),
            anomalies.join(", ")
        ));
    }

    let range = (max_temp - min_temp).max(0.1);
    let mut bins = vec![0.0_f64; 256];
    for temp in &window {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let idx = (((temp - min_temp) / range) * 255.0).round() as usize;
        let idx = idx.min(255);
        bins[idx] += 1.0;
    }

    flow_output!("report" => json!(report), "bins" => json!(bins))
}
