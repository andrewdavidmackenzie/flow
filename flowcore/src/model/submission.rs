use std::fmt;
use std::time::Duration;

use log::info;
use serde_derive::{Deserialize, Serialize};
use url::Url;

/// A `Submission` is the struct used to send a flow to the Coordinator for execution. It contains
/// all the information necessary to execute it:
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Submission {
    /// The URL where the manifest of the flow to execute can be found
    pub manifest_url: Url,
    /// An optional maximum number of jobs you want dispatched/executing in parallel
    pub max_parallel_jobs: Option<usize>,
    /// The Duration to wait before timing out when waiting for jobs to complete
    pub job_timeout: Duration,
    /// Whether to debug the flow while executing it
    #[cfg(feature = "debugger")]
    pub debug: bool,
}

impl Submission {
    /// Create a new `Submission` of a `Flow` for execution with the specified `Manifest`
    /// of `Functions`, optionally setting a limit for the number of jobs running in parallel
    /// via `max_parallel_jobs`
    pub fn new(
        manifest_url: &Url,
        max_parallel_jobs: Option<usize>,
        #[cfg(feature = "debugger")] debug: bool,
    ) -> Submission {
        if let Some(limit) = max_parallel_jobs {
            info!("Maximum jobs in parallel limited to {limit}");
        }

        Submission {
            manifest_url: manifest_url.to_owned(),
            max_parallel_jobs,
            job_timeout: Duration::from_secs(60),
            #[cfg(feature = "debugger")]
            debug,
        }
    }
}
impl fmt::Display for Submission {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Submission:")?;
        writeln!(f, "         Manifest URL: {}", self.manifest_url)?;
        if let Some(limit) = self.max_parallel_jobs {
            writeln!(f, "Maximum Parallel Jobs: {limit}")?;
        }
        write!(f,   "          Job Timeout: {:?}", self.job_timeout)
    }
}
