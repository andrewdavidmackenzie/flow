use std::fmt;

use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use flowcore::errors::*;
use flowcore::model::output_connection::OutputConnection;
use flowcore::RunAgain;

/// A `Job` contains the information necessary to manage the execution of a function in the
/// flow on a set of input values, and then where to send the outputs that maybe produces.
#[derive(Serialize, Deserialize, Clone)]
pub struct Job {
    /// Each `Job` has a unique id that increments as jobs are executed
    pub job_id: usize,
    /// The `id` of the function in the `RunState`'s list of functions that will execute this job
    pub function_id: usize,
    /// The `id` of the nested flow (from root flow on down) there the function executing the job is
    pub flow_id: usize,
    /// The set of input values to be used by the function when executing this job
    pub input_set: Vec<Value>,
    /// The destinations (other function's inputs) where any output should be sent
    pub connections: Vec<OutputConnection>,
    /// The url of the implementation to be run for this job
    pub implementation_url: Url,
    /// The result of the execution with optional output Value and if the function should be run
    /// again in the future
    pub result: Result<(Option<Value>, RunAgain)>,
}

unsafe impl Send for Job{}
unsafe impl Sync for Job{}

impl fmt::Display for Job {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "Job Id: {}, Function Id: {}, Flow Id: {}",
            self.job_id, self.function_id, self.flow_id
        )?;
        writeln!(f, "Implementation Url: {}", self.implementation_url)?;
        writeln!(f, "Inputs: {:?}", self.input_set)?;
        writeln!(f, "Connections: {:?}", self.connections)?;
        write!(f, "Result: {:?}", self.result)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use serde_json::json;
    use url::Url;

    use flowcore::model::datatype::ARRAY_TYPE;

    #[test]
    fn display_job_test() {
        let job = super::Job {
            job_id: 0,
            function_id: 1,
            flow_id: 0,
            input_set: vec![],
            connections: vec![],
            implementation_url: Url::parse("lib://flowstdlib/math/add").expect("Could not parse Url"),
            result: Ok((None, false)),
        };
        println!("Job: {job}");
    }

    #[test]
    fn get_entire_output_value() {
        let job = super::Job {
            job_id: 0,
            function_id: 1,
            flow_id: 0,
            input_set: vec![],
            connections: vec![],
            implementation_url: Url::parse("lib://flowstdlib/math/add").expect("Could not parse Url"),
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
            job_id: 0,
            function_id: 1,
            flow_id: 0,
            input_set: vec![],
            connections: vec![],
            implementation_url: Url::parse("lib://flowstdlib/math/add").expect("Could not parse Url"),
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
