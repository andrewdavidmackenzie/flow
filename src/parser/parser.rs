extern crate yaml_rust;

use self::yaml_rust::{YamlLoader, Yaml, scanner};

use description::context::Context;
use description::flow::Flow;
use description::io::IOSet;

use parser::yaml;

// TODO Error reporting properly and implement the Error trait
pub enum Result  {
    ContextLoaded(Context),
    FlowLoaded(Flow),
    Error(String),
    Valid,
}

/// # Example
/// ```
/// use flow::parser::parser;
///
/// parser::load("samples/hello-world-simple/hello.context", true);
/// ```
pub fn load(path: &str, context_allowed: bool) -> Result {
    info!("Attempting to load Yaml from: {}", path);

    let parseResult = yaml::load(path, context_allowed);

    match parseResult {
        Result::ContextLoaded(mut context) => {
            if cfg!(not(ndebug)) {
                info!("Validating context: {}", context.name);
            }
            context.validate_fields(); // TODO early return
            context.load_sub_flows(); // TODO early return
            context.validate_connections(); // TODO early return
            for &(_, _, ref subflow) in context.flows.iter() {
                subflow.borrow_mut().subflow(); // TODO early return
            }
            return Result::ContextLoaded(context);
        },

        Result::FlowLoaded(mut flow) => {
            if cfg!(not(ndebug)) {
                info!("Validating flow: {}", flow.name);
            }
            flow.validate_fields(); // TODO early return
            flow.load_sub_flows(); // TODO early return
            flow.validate_connections(); // TODO early return
            flow.subflow();
            return Result::FlowLoaded(flow);
        },

        Result::Error(string) => {
            if cfg!(not(ndebug)) {
                error!("Error loading Yaml: {}", string);
            }
            return Result::Error(string);
        },

        Result::Valid => {
            if cfg!(not(ndebug)) {
                error!("Unexpected 'Valid' while loading Yaml");
            }
            return Result::Error("Neither a context nor a flow was found".to_string());
        }
    }
}