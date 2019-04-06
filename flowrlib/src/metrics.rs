use std::time::Instant;
use std::fmt;
use std::cmp::max;

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
        write!(f, "\t Number of Functions: \t{}\n", self.num_functions)?;
        write!(f, "\t        Outputs sent: \t{}\n", self.outputs_sent)?;
        write!(f, "\t     Elapsed time(s): \t{:.*}\n", 6, elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 * 1e-9)?;
        write!(f, "\tMax Jobs in Parallel: \t{}", self.max_simultaneous_jobs)
    }
}

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
