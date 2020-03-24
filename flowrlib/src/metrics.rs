use std::cmp::max;
use std::fmt;

use instant::Instant;

pub struct Metrics {
    num_functions: usize,
    outputs_sent: u32,
    start_time: Instant,
    max_simultaneous_jobs: usize,
}

impl Metrics {
    pub fn new(num_functions: usize) -> Self {
        Metrics {
            num_functions,
            outputs_sent: 0,
            start_time: Instant::now(),
            max_simultaneous_jobs: 0,
        }
    }

    pub fn reset(&mut self) {
        self.outputs_sent = 0;
        self.start_time = Instant::now();
        self.max_simultaneous_jobs = 0;
    }

    pub fn increment_outputs_sent(&mut self) {
        self.outputs_sent += 1;
    }

    pub fn track_max_jobs(&mut self, jobs_running: usize) {
        self.max_simultaneous_jobs = max(self.max_simultaneous_jobs, jobs_running);
    }
}

impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let elapsed = self.start_time.elapsed();
        write!(f, "\t   Number of Functions: {}\n", self.num_functions)?;
        write!(f, "\t          Outputs sent: {}\n", self.outputs_sent)?;
        write!(f, "\t       Elapsed time(s): {:.*}\n", 6, elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9)?;
        write!(f, "\t  Max Jobs in Parallel: {}", self.max_simultaneous_jobs)
    }
}

#[cfg(test)]
mod test {
    use super::Metrics;

    #[test]
    fn test_metrics_reset() {
        let mut metrics = Metrics::new(10);
        metrics.outputs_sent = 10;
        metrics.max_simultaneous_jobs = 4;
        metrics.reset();
        assert_eq!(metrics.outputs_sent, 0);
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
        println!("{}", metrics);
    }
}