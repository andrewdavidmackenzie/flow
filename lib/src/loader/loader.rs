use description::context::Context;
use description::flow::Flow;
//use description::io::IOSet;
use std::result;
use std::fs::File;
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
/// use std::fs::File;
/// use flowlib::loader::loader;
///
/// let path = "../samples/hello-world-simple/hello.context";
/// let mut file = File::open(path).unwrap();
/// loader::load(file);
///
/// ```
pub fn load(file: File) -> Result {
    // We only support Yaml for now...
    match yaml::load(file) {
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