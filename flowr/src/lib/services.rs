/// `JOB_SERVICE_NAME` can be used to discover the queue serving jobs for execution
pub const JOB_SERVICE_NAME: &str = "jobs._flowr._tcp.local";

/// `RESULTS_JOB_SERVICE_NAME` can be used to discover the queue where to send job results
pub const RESULTS_JOB_SERVICE_NAME: &str = "results._flowr._tcp.local";

/// `CONTROL_SERVICE_NAME` is a control PUB/SUB socket used to control executors that
/// are listening on the `JOB_SERVICE` and sending results back via the `RESULTS_SERVICE`
pub const CONTROL_SERVICE_NAME: &str = "control._flowr._tcp.local";

/// This is the port for announcing and discovering the job queues
pub const JOB_QUEUES_DISCOVERY_PORT:u16 = 15003;
