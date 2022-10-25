use std::cmp::max;
use std::fmt;
use std::time::Instant;

use log::debug;
use serde_derive::{Deserialize, Serialize};

/// `Metrics` stacks a number of statistics on flow execution while being executed
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Metrics {
    num_functions: usize,
    jobs_created: usize,
    outputs_sent: u32,
    #[serde(skip)]
    #[serde(default = "Metrics::default_start_time")]
    start_time: Instant,
    max_simultaneous_jobs: usize,
}

impl Metrics {
    /// Create a new `Metrics` struct
    pub fn new(num_functions: usize) -> Self {
        Metrics {
            num_functions,
            jobs_created: 0,
            outputs_sent: 0,
            start_time: Instant::now(),
            max_simultaneous_jobs: 0,
        }
    }

    /// Reset the values of a `Metrics` struct back to their initial values
    pub fn reset(&mut self) {
        debug!("Resetting Metrics");
        self.jobs_created = 0;
        self.outputs_sent = 0;
        self.start_time = Instant::now();
        self.max_simultaneous_jobs = 0;
    }

    /// Set the number of jobs created in `Metrics` to the `jobs` value
    pub fn set_jobs_created(&mut self, jobs: usize) {
        self.jobs_created = jobs;
    }

    /// Increment the tracker for the number of output values sent between functions
    pub fn increment_outputs_sent(&mut self) {
        self.outputs_sent += 1;
    }

    /// Keep track of the maximum jobs that are executing in parallel during a flows
    /// execution, as a measure of the maximum level of parallelism achieved
    pub fn track_max_jobs(&mut self, jobs_running: usize) {
        self.max_simultaneous_jobs = max(self.max_simultaneous_jobs, jobs_running);
    }

    /// Return the start time for flow execution - used for tracking wall-clock time for
    /// the execution
    pub fn default_start_time() -> Instant {
        Instant::now()
    }
}

impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let elapsed = self.start_time.elapsed();
        writeln!(f, "\t   Number of Functions: {}", self.num_functions)?;
        writeln!(f, "\tNumber of Jobs Created: {}", self.jobs_created)?;
        writeln!(f, "\t           Values sent: {}", self.outputs_sent)?;
        writeln!(
            f,
            "\t       Elapsed time(s): {:.*}",
            6,
            elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9
        )?;
        write!(
            f,
            "\t  Max Jobs in Parallel: {}",
            self.max_simultaneous_jobs
        )
    }
}

#[cfg(test)]
mod test {
    use super::Metrics;

    #[test]
    fn test_metrics_reset() {
        let mut metrics = Metrics::new(10);
        metrics.jobs_created = 110;
        metrics.outputs_sent = 10;
        metrics.max_simultaneous_jobs = 4;
        metrics.reset();
        assert_eq!(metrics.outputs_sent, 0);
        assert_eq!(metrics.jobs_created, 0);
        assert_eq!(metrics.num_functions, 10);
        assert_eq!(metrics.max_simultaneous_jobs, 0);
    }

    #[test]
    fn test_max_tracking() {
        let mut metrics = Metrics::new(10);
        assert_eq!(metrics.max_simultaneous_jobs, 0);

        metrics.track_max_jobs(2);
        metrics.track_max_jobs(4);
        metrics.track_max_jobs(3);

        assert_eq!(metrics.max_simultaneous_jobs, 4);
    }

    #[test]
    fn test_metrics_display() {
        let metrics = Metrics::new(10);
        println!("{metrics}");
    }
}
