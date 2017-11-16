use description::context::Context;
use description::flow::Flow;
use std::result;
use std::path::PathBuf;
use loader::yaml;

pub enum Result  {
    Context(Context),
    Flow(Flow),
    Error(String)
}

pub trait Validate {
    fn validate(&self) -> result::Result<(), String>;
}

/// # Example
/// ```
/// use std::path::PathBuf;
/// use flowlib::loader::loader;
///
/// let path = PathBuf::from("../samples/hello-world-simple/hello.context");
/// loader::load(path);
///
/// ```
pub fn load(file_path: PathBuf) -> Result {
    // We only support Yaml for now...
    match yaml::load(file_path) {
        Result::Context(context) => {
            match context.validate() {
                Ok(_) => Result::Context(context),
                Err(e) => Result::Error(e)
            }
        },

        Result::Flow(flow) => {
            return Result::Flow(flow);
        },

        Result::Error(string) => {
            return Result::Error(string);
        }
    }
}