use std::fmt;
use std::time::Duration;

use log::info;
use serde_derive::{Deserialize, Serialize};

use crate::model::flow_manifest::FlowManifest;

/// A `Submission` is the struct used to send a flow to the Coordinator for execution. It contains
/// all the information necessary to execute it:
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Submission {
    /// The FlowManifest loaded from the manifest_url
    pub manifest: FlowManifest,

    /// An optional limit on the number of jobs that can be dispatched for execution in parallel
    pub parallel_jobs_limit: Option<usize>,
    /// The Duration to wait before timing out when waiting for jobs to complete
    pub job_timeout: Duration,
    /// Whether to debug the flow while executing it
    #[cfg(feature = "debugger")]
    pub debug: bool,
}

impl Submission {
    /// Create a new `Submission` of a flow for execution with the specified `FlowManifest`
    /// optionally setting a limit for the number of jobs running in parallel
    /// via `max_parallel_jobs`
    pub fn new(
        manifest: FlowManifest,
        parallel_jobs_limit: Option<usize>,
        #[cfg(feature = "debugger")] debug: bool,
    ) -> Submission {
        if let Some(limit) = parallel_jobs_limit {
            info!("Maximum jobs in parallel limited to {limit}");
        }

        Submission {
            manifest,
            parallel_jobs_limit,
            job_timeout: Duration::from_secs(60),
            #[cfg(feature = "debugger")]
            debug,
        }
    }
}
impl fmt::Display for Submission {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Parallel Jobs Limit: {:?}", self.parallel_jobs_limit)?;
        writeln!(f,   "          Job Timeout: {:?}", self.job_timeout)?;
        #[cfg(feature = "debugger")]
        writeln!(f,   "                Debug: {}", self.debug)?;
        write!(f,     "             Manifest: {}", self.manifest)
    }
}
