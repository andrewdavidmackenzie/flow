/// The mDNS service type for flow services
pub const FLOW_SERVICE_TYPE: &str = "_flowr._tcp.local.";

/// `JOB_SERVICE_NAME` can be used to discover the queue serving jobs for execution
pub const JOB_SERVICE_NAME: &str = "jobs";

/// `RESULTS_JOB_SERVICE_NAME` can be used to discover the queue where to send job results
pub const RESULTS_JOB_SERVICE_NAME: &str = "results";

/// `CONTROL_SERVICE_NAME` is a control PUB/SUB socket used to control executors that
/// are listening on the `JOB_SERVICE` and sending results back via the `RESULTS_SERVICE`
pub const CONTROL_SERVICE_NAME: &str = "control";

/// Use this to discover the coordinator service by name
pub const COORDINATOR_SERVICE_NAME: &str = "runtime";

/// Use this to discover the debug service by name
#[cfg(feature = "debugger")]
pub const DEBUG_SERVICE_NAME: &str = "debug";
