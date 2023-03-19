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
    /// An optional maximum number of jobs you want dispatched/executing in parallel
    pub max_parallel_jobs: Option<usize>,
    /// The Duration to wait before timing out when waiting for jobs to complete
    pub job_timeout: Option<Duration>,
    /// Whether debugging is enabled or not for the flow
    #[cfg(feature = "debugger")]
    pub debug_enabled: bool,
}

impl Submission {
    /// Create a new `Submission` of a flow for execution with the specified `FlowManifest`
    /// optionally setting a limit for the number of jobs running in parallel
    /// via `max_parallel_jobs`
    pub fn new(
        manifest: FlowManifest,
        max_parallel_jobs: Option<usize>,
        job_timeout: Option<Duration>,
        #[cfg(feature = "debugger")] debug: bool,
    ) -> Submission {
        if let Some(limit) = max_parallel_jobs {
            info!("Maximum jobs in parallel limited to {limit}");
        }

        Submission {
            manifest,
            max_parallel_jobs,
            job_timeout,
            #[cfg(feature = "debugger")]
            debug_enabled: debug,
        }
    }
}
impl fmt::Display for Submission {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(limit) = self.max_parallel_jobs {
            writeln!(f, "Maximum Parallel Jobs: {limit}")?;
        }
        writeln!(f,   "          Job Timeout: {:?}", self.job_timeout)?;
        #[cfg(feature = "debugger")]
        writeln!(f,   "                Debug: {}", self.debug_enabled)?;
        write!(f,     "             Manifest: \n{}", self.manifest)
    }
}
