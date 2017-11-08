extern crate yaml_rust;

use self::yaml_rust::{YamlLoader, Yaml, scanner};

use description::context::Context;
use description::flow::Flow;
use description::io::IOSet;

use parser::yaml;

pub enum Result<'a>  {
    ContextLoaded(Context<>),
    FlowLoaded(Flow<'a>),
    Valid,
    Error(String)
}

/// # Example
/// ```
/// use flowlib::parser::parser;
///
/// parser::load("samples/hello-world-simple/hello.context", true);
/// ```
pub fn load(path: &str, context_allowed: bool) -> Result {
    // For now, we only support loading from a yaml file
    info!("Attempting to load Yaml from: {}", path);
    let parseResult = yaml::load(path, context_allowed);

    match parseResult {
        Result::ContextLoaded(mut context) => {
            //            info!("Validating context: {}", context.name);
            //            context.validate_fields(); // TODO early return
            //            context.load_sub_flows(); // TODO early return
            //            context.validate_connections(); // TODO early return
            //            for &(_, _, ref subflow) in context.flows.iter() {
            //                subflow.borrow_mut().subflow(); // TODO early return
            //            }
            return Result::ContextLoaded(context);
        },

        Result::FlowLoaded(mut flow) => {
            info!("Validating flow: {}", flow.name);
            //            flow.validate_fields(); // TODO early return
            //            flow.load_sub_flows(); // TODO early return
            //            flow.validate_connections(); // TODO early return
            //            flow.subflow();
            return Result::FlowLoaded(flow);
        },

        Result::Error(string) => {
            if cfg!(not(ndebug)) {
                error!("Error loading Yaml: {}", string);
            }
            return Result::Error(string);
        },

        Result::Valid => {
            return Result::Error("Ask to load not validate!".to_string());
        }
    }
}