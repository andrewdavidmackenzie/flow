use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use flowcore::errors::Result;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};

/// `Implementation` struct for the `time/get` context function.
///
/// Returns the current system time and the elapsed milliseconds since the
/// previous call. The input is ignored (used only as a trigger).
///
/// Output is an object with keys:
/// - `"time"` — the current time in ISO 8601 format
/// - `"epoch_secs"` — seconds since epoch
/// - `"epoch_millis"` — milliseconds since epoch
/// - `"elapsed_millis"` — milliseconds since the previous invocation (0 on first call)
#[derive(Debug)]
pub struct Get {
    previous_millis: Mutex<u64>,
}

impl Get {
    /// Create a new `Get` instance
    #[must_use]
    pub fn new() -> Self {
        Self {
            previous_millis: Mutex::new(0),
        }
    }
}

impl Implementation for Get {
    fn run(&self, _inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let now = SystemTime::now();
        let duration = now
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("System time before Unix epoch: {e}"))?;

        let epoch_secs = duration.as_secs();
        #[allow(clippy::cast_possible_truncation)]
        let epoch_millis = duration.as_millis() as u64;

        let elapsed_millis = if let Ok(mut prev) = self.previous_millis.lock() {
            let elapsed = if *prev == 0 {
                0
            } else {
                epoch_millis.saturating_sub(*prev)
            };
            *prev = epoch_millis;
            elapsed
        } else {
            0
        };

        let iso = epoch_to_iso8601(epoch_secs);

        let output = json!({
            "time": iso,
            "epoch_secs": epoch_secs,
            "epoch_millis": epoch_millis,
            "elapsed_millis": elapsed_millis,
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
    fn get_time_returns_all_fields() {
        let time_fn = Get::new();
        let (output, run_again) = time_fn.run(&[json!("now")]).expect("run failed");
        assert!(run_again, "Should run again");
        let value = output.expect("Should have output");
        assert!(value.get("time").is_some());
        assert!(value.get("epoch_secs").is_some());
        assert!(value.get("epoch_millis").is_some());
        assert!(value.get("elapsed_millis").is_some());
    }

    #[test]
    fn first_call_elapsed_is_zero() {
        let time_fn = Get::new();
        let (output, _) = time_fn.run(&[json!("now")]).expect("run failed");
        let value = output.expect("Should have output");
        assert_eq!(value["elapsed_millis"].as_u64().unwrap(), 0);
    }

    #[test]
    fn second_call_elapsed_is_non_negative() {
        let time_fn = Get::new();
        let _ = time_fn.run(&[json!("now")]).expect("run failed");
        let (output, _) = time_fn.run(&[json!("now")]).expect("run failed");
        let value = output.expect("Should have output");
        // elapsed should be >= 0 (likely 0 for back-to-back calls)
        assert!(value["elapsed_millis"].as_u64().is_some());
    }

    #[test]
    fn iso8601_format() {
        let time_fn = Get::new();
        let (output, _) = time_fn.run(&[json!("now")]).expect("run failed");
        let value = output.expect("Should have output");
        let time_str = value["time"].as_str().expect("time should be a string");
        assert!(time_str.ends_with('Z'));
        assert!(time_str.contains('T'));
    }

    #[test]
    fn no_input_works() {
        let time_fn = Get::new();
        let (output, _) = time_fn.run(&[]).expect("run failed");
        assert!(output.is_some());
    }

    #[test]
    fn epoch_to_iso8601_known_value() {
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
        let time_fn = Get::new();
        let (output, _) = time_fn.run(&[json!("now")]).expect("run failed");
        let value = output.expect("Should have output");
        let secs = value["epoch_secs"].as_u64().unwrap();
        let millis = value["epoch_millis"].as_u64().unwrap();
        assert!(millis >= secs * 1000);
    }
}
