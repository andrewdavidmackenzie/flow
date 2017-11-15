extern crate yaml_rust;

use self::yaml_rust::{YamlLoader, Yaml, scanner};

use description::context::Context;
use description::flow::Flow;
use description::io::IOSet;
use std::result;
use std::fs::File;
use parser::yaml;

pub enum Result  {
    ContextLoaded(Context),
    FlowLoaded(Flow),
    Error(String)
}

pub trait Validate {
    fn validate(&self) -> result::Result<(), String>;
}

/*

validate model (see check)

load flow definition from file specified in arguments
    - load any referenced to included flows also

construct overall list of functions

construct list of connections

construct initial list of all functions able to produce output
    - start from external sources at level 0

do
    - identify all functions which receive input from active sources
    - execute all those functions
    - functions producing output added to list of active sources
while functions pending input

 */

/// # Example
/// ```
/// use std::fs::File;
/// use flowlib::parser::parser;
///
/// let path = "../samples/hello-world-simple/hello.context";
/// let mut file = File::open(path).unwrap();
/// parser::load(file);
///
/// ```
pub fn load(file: File) -> Result {
    // We only support Yaml for now...
    match yaml::load(file) {
        Result::ContextLoaded(context) => {
            //            info!("Validating context: {}", context.name);
            //            context.validate(); // TODO early return
            //            context.load_sub_flows(); // TODO early return
            //            context.validate_connections(); // TODO early return
            //            for &(_, _, ref subflow) in context.flows.iter() {
            //                subflow.borrow_mut().subflow(); // TODO early return
            //            }
            return Result::ContextLoaded(context);
        },

        Result::FlowLoaded(flow) => {
            //            flow.validate(); // TODO early return
            //            flow.load_sub_flows(); // TODO early return
            //            flow.validate_connections(); // TODO early return
            //            flow.subflow();
            return Result::FlowLoaded(flow);
        },

        Result::Error(string) => {
            return Result::Error(string);
        }
    }
}