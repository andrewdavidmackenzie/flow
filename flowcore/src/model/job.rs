use std::fmt;

use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use crate::errors::Result;
use crate::model::output_connection::OutputConnection;
use crate::RunAgain;

/// Conatins the minimum amount of information required to execute a [Job] and return the result
#[derive(Serialize, Deserialize, Clone)]
pub struct Payload {
    /// Each `Job` has a unique id that increments as jobs are executed
    pub job_id: usize,
    /// The set of input values to be used by the function when executing this job
    pub input_set: Vec<Value>,
    /// The url of the implementation to be run for this job
    pub implementation_url: Url,
}

/// A `Job` contains the information necessary to manage the execution of a function in the
/// flow on a set of input values, and then where to send the outputs that maybe produces.
#[derive(Serialize, Deserialize, Clone)]
pub struct Job {
    /// The `process_id` of the function in the `RunState`'s list of functions that will execute this job
    pub process_id: usize,
    /// The `parent_id` of the flow containing the function executing the job
    pub parent_id: usize,
    /// The function name (for logging without looking up the function)
    #[cfg(feature = "debugger")]
    pub function_name: String,
    /// the payload required to execute the job
    pub payload: Payload,
    /// The result of the execution with the `job_id`, the optional output Value and if the function
    /// should be run again in the future
    pub result: Result<(Option<Value>, RunAgain)>,
    /// The destinations (other function's inputs) where any output should be sent
    pub connections: Vec<OutputConnection>,
}

impl fmt::Display for Job {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.payload)?;
        writeln!(f, "Connections: {:?}", self.connections)?;
        writeln!(
            f,
            "Process Id: {}, Parent Id: {}",
            self.process_id, self.parent_id
        )?;
        write!(f, "Result: {:?}", self.result)
    }
}

impl fmt::Display for Payload {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Job #: {}", self.job_id)?;
        writeln!(f, "Implementation Url: {}", self.implementation_url)?;
        writeln!(f, "Inputs: {:?}", self.input_set)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use std::collections::HashMap;

    use serde_json::json;
    use url::Url;

    use crate::model::datatype::ARRAY_TYPE;

    use crate::model::job::Payload;

    #[test]
    fn display_job_test() {
        let job = super::Job {
            process_id: 1,
            #[cfg(feature = "debugger")]
            function_name: String::new(),
            parent_id: 0,
            connections: vec![],
            payload: Payload {
                job_id: 0,
                input_set: vec![],
                implementation_url: Url::parse("lib://flowstdlib/math/add")
                    .expect("Could not parse Url"),
            },
            result: Ok((None, false)),
        };
        println!("Job: {job}");
    }

    #[test]
    fn get_entire_output_value() {
        let job = super::Job {
            process_id: 1,
            #[cfg(feature = "debugger")]
            function_name: String::new(),
            parent_id: 0,
            connections: vec![],
            payload: Payload {
                job_id: 0,
                input_set: vec![],
                implementation_url: Url::parse("lib://flowstdlib/math/add")
                    .expect("Could not parse Url"),
            },
            result: Ok((Some(json!(42u64)), false)),
        };

        assert_eq!(
            &json!(42u64),
            job.result
                .expect("Could not get result")
                .0
                .expect("No output value when one was expected")
                .pointer("")
                .expect("Could not get value using json pointer")
        );
    }

    #[test]
    fn get_sub_array_from_output_value() {
        let mut map = HashMap::new();
        map.insert(ARRAY_TYPE, vec![1, 2, 3]);
        let value = json!(map);
        let job = super::Job {
            process_id: 1,
            #[cfg(feature = "debugger")]
            function_name: String::new(),
            parent_id: 0,
            connections: vec![],
            payload: Payload {
                job_id: 0,
                input_set: vec![],
                implementation_url: Url::parse("lib://flowstdlib/math/add")
                    .expect("Could not parse Url"),
            },
            result: Ok((Some(json!(value)), false)),
        };

        assert_eq!(
            &json!(3),
            job.result
                .expect("Could not get result")
                .0
                .expect("No output value when one was expected")
                .pointer("/array/2")
                .expect("Could not get value using json pointer")
        );
    }
}
