use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use flowcore::errors::Result;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

/// `Implementation` struct for the `time/get` context function.
///
/// Returns the current system time. The input specifies the format:
/// - `"now"` or `"iso8601"` — returns the current time in ISO 8601 format
/// - `"epoch_secs"` — returns seconds since the Unix epoch as a number
/// - `"epoch_millis"` — returns milliseconds since the Unix epoch as a number
/// - any other string — treated as `"iso8601"`
///
/// Output is an object with keys:
/// - `"time"` — the formatted time string or number
/// - `"epoch_secs"` — seconds since epoch (always included as a number)
/// - `"epoch_millis"` — milliseconds since epoch (always included as a number)
#[derive(Debug)]
pub struct Get;

impl Implementation for Get {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let format = inputs.first().and_then(Value::as_str).unwrap_or("iso8601");

        let now = SystemTime::now();
        let duration = now
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("System time before Unix epoch: {e}"))?;

        let epoch_secs = duration.as_secs();
        let epoch_millis = duration.as_millis();

        let time_value = match format {
            "epoch_secs" => json!(epoch_secs),
            "epoch_millis" => json!(epoch_millis),
            _ => {
                // ISO 8601 format: construct from epoch
                let iso = epoch_to_iso8601(epoch_secs);
                json!(iso)
            }
        };

        let output = json!({
            "time": time_value,
            "epoch_secs": epoch_secs,
            "epoch_millis": epoch_millis,
        });

        Ok((Some(output), RUN_AGAIN))
    }
}

/// Convert Unix epoch seconds to an ISO 8601 formatted string (UTC).
///
/// Produces format: `YYYY-MM-DDTHH:MM:SSZ`
fn epoch_to_iso8601(epoch_secs: u64) -> String {
    // Days calculation from Unix epoch (1970-01-01)
    let total_days = epoch_secs / 86400;
    let time_of_day = epoch_secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    let (year, month, day) = days_to_ymd(total_days);

    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

/// Convert total days since 1970-01-01 to (year, month, day).
fn days_to_ymd(total_days: u64) -> (u64, u64, u64) {
    // Algorithm based on Howard Hinnant's civil_from_days
    let z = total_days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod test {
    use serde_json::json;

    use flowcore::Implementation;

    use super::Get;

    #[test]
    fn get_time_now_returns_iso8601() {
        let time_fn = Get;
        let (output, run_again) = time_fn.run(&[json!("now")]).expect("run failed");
        assert!(run_again, "Should run again");
        let value = output.expect("Should have output");
        assert!(value.get("time").is_some(), "Should have 'time' key");
        assert!(
            value.get("epoch_secs").is_some(),
            "Should have 'epoch_secs'"
        );
        assert!(
            value.get("epoch_millis").is_some(),
            "Should have 'epoch_millis'"
        );
        // time should be a string in ISO format
        let time_str = value["time"].as_str().expect("time should be a string");
        assert!(time_str.ends_with('Z'), "Should end with Z for UTC");
        assert!(time_str.contains('T'), "Should contain T separator");
    }

    #[test]
    fn get_time_iso8601_format() {
        let time_fn = Get;
        let (output, _) = time_fn.run(&[json!("iso8601")]).expect("run failed");
        let value = output.expect("Should have output");
        let time_str = value["time"].as_str().expect("time should be a string");
        assert!(time_str.ends_with('Z'));
    }

    #[test]
    fn get_time_epoch_secs() {
        let time_fn = Get;
        let (output, _) = time_fn.run(&[json!("epoch_secs")]).expect("run failed");
        let value = output.expect("Should have output");
        assert!(
            value["time"].as_u64().is_some(),
            "epoch_secs time should be a number"
        );
        assert!(
            value["epoch_secs"].as_u64().unwrap() > 0,
            "epoch_secs should be positive"
        );
    }

    #[test]
    fn get_time_epoch_millis() {
        let time_fn = Get;
        let (output, _) = time_fn.run(&[json!("epoch_millis")]).expect("run failed");
        let value = output.expect("Should have output");
        let millis = value["time"].as_u64().expect("Should be a number");
        assert!(millis > 1_000_000_000_000, "Should be in milliseconds");
    }

    #[test]
    fn get_time_default_format() {
        let time_fn = Get;
        // No input — defaults to iso8601
        let (output, _) = time_fn.run(&[]).expect("run failed");
        let value = output.expect("Should have output");
        assert!(
            value["time"].as_str().is_some(),
            "Default format should produce a string"
        );
    }

    #[test]
    fn get_time_unknown_format_defaults_to_iso() {
        let time_fn = Get;
        let (output, _) = time_fn.run(&[json!("unknown_format")]).expect("run failed");
        let value = output.expect("Should have output");
        let time_str = value["time"].as_str().expect("Should be a string");
        assert!(time_str.ends_with('Z'));
    }

    #[test]
    fn epoch_to_iso8601_known_value() {
        // 2024-01-01T00:00:00Z = 1704067200 seconds since epoch
        let iso = super::epoch_to_iso8601(1_704_067_200);
        assert_eq!(iso, "2024-01-01T00:00:00Z");
    }

    #[test]
    fn epoch_to_iso8601_epoch_zero() {
        let iso = super::epoch_to_iso8601(0);
        assert_eq!(iso, "1970-01-01T00:00:00Z");
    }

    #[test]
    fn epoch_millis_greater_than_secs() {
        let time_fn = Get;
        let (output, _) = time_fn.run(&[json!("now")]).expect("run failed");
        let value = output.expect("Should have output");
        let secs = value["epoch_secs"].as_u64().unwrap();
        let millis = value["epoch_millis"].as_u64().unwrap();
        assert!(millis >= secs * 1000);
    }
}
