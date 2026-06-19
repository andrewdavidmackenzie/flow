use std::f64::consts::PI;

use flowcore::errors::Result;
use flowcore::{RUN_AGAIN, RunAgain};
use flowmacro::flow_function;
use serde_json::{json, Value};

fn fft(signal: &[f64]) -> Vec<(f64, f64)> {
    let n = signal.len();
    let mut result = Vec::with_capacity(n);

    for k in 0..n {
        let mut re = 0.0;
        let mut im = 0.0;
        for (t, sample) in signal.iter().enumerate() {
            let angle = 2.0 * PI * (k as f64) * (t as f64) / (n as f64);
            re += sample * angle.cos();
            im -= sample * angle.sin();
        }
        result.push((re, im));
    }

    result
}

#[flow_function]
fn fft_compute(inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
    let signal: Vec<f64> =
        serde_json::from_value(inputs.get(0).ok_or("missing input: signal")?.clone())?;
    let sample_rate: f64 =
        serde_json::from_value(inputs.get(1).ok_or("missing input: sample_rate")?.clone())?;

    if signal.is_empty() {
        return Err("signal must contain at least one sample".into());
    }
    if sample_rate <= 0.0 {
        return Err("sample_rate must be > 0".into());
    }

    let spectrum = fft(&signal);
    let n = spectrum.len();

    let magnitudes: Vec<(f64, f64)> = spectrum
        .iter()
        .enumerate()
        .take(n / 2)
        .map(|(i, (re, im))| {
            let freq = (i as f64) * sample_rate / (n as f64);
            let mag = (re * re + im * im).sqrt() / (n as f64);
            (freq, mag)
        })
        .collect();

    let max_mag = magnitudes.iter().map(|(_, m)| *m).fold(0.0_f64, f64::max);

    let bar_width = 40;
    let mut text = String::from("FFT Spectrum\n");
    text.push_str(&format!("Signal: {} samples at {} Hz\n\n", n, sample_rate));
    text.push_str(&format!("{:>8}  {:<40}  {}\n", "Freq", "Magnitude", ""));
    text.push_str(&format!("{:>8}  {:<40}  {}\n", "----", &"-".repeat(bar_width), ""));

    for (freq, mag) in &magnitudes {
        if *mag > max_mag * 0.02 {
            let bar_len = if max_mag > 0.0 {
                (mag / max_mag * bar_width as f64).round() as usize
            } else {
                0
            };
            let bar: String = "\u{2588}".repeat(bar_len);
            text.push_str(&format!("{:>7.0} Hz  {:<40}  {:.2}\n", freq, bar, mag));
        }
    }

    let bins: Vec<f64> = magnitudes.iter().map(|(_, mag)| *mag).collect();

    let output = json!({
        "text": text,
        "bins": bins
    });

    Ok((Some(output), RUN_AGAIN))
}
