extern crate yaml_rust;

use self::yaml_rust::{YamlLoader, Yaml, scanner};

use description::context::Context;
use description::flow::Flow;
use description::io::IOSet;
use description::connection::ConnectionSet;

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
/// use flow::parser::load;
///
/// flow::parser::load("hello.context", true);
/// ```
pub fn load(path: &str, context_allowed: bool) -> Result {
	let parseResult = yaml::load(path, context_allowed);

	match parseResult {
		Result::ContextLoaded(mut context) => {
			context.validate_fields(); // TODO early return
            context.loadSubflows(); // TODO early return
            context.validateConnections(); // TODO early return
			for &(_, _, ref subflow) in context.flows.iter() {
            	subflow.borrow_mut().subflow(); // TODO early return
			}
//				let (_, _, subflow) = &context.flows[0];
			return Result::Valid;
		},

		Result::FlowLoaded(mut flow) => {
			flow.validate_fields(); // TODO early return
            flow.loadSubflows(); // TODO early return
            flow.validateConnections(); // TODO early return
            flow.subflow();
			return Result::Valid;
		},

		Result::Error(string) => return Result::Error(string),

        Result::Valid => return Result::Error("Shouldn't happen".to_string()),
	}

	Result::Valid
}