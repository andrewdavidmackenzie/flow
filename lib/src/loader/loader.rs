use description::context::Context;
use description::flow::Flow;
use std::result;
use std::path::PathBuf;
use loader::yaml_loader::FlowYamlLoader;
use loader::toml_loader::FlowTomelLoader;

pub enum Result  {
    Context(Context),
    Flow(Flow),
    Error(String)
}

pub trait Validate {
    fn validate(&self) -> result::Result<(), String>;
}

pub trait Loader {
    fn load(file_path: &PathBuf) -> self::Result;
}

const CONTEXT_TAGS: &'static [&'static str] = &["context", "entities", "values", "flow", "connection_set"];
const FLOW_TAGS: &'static [&'static str] = &["flow", "ioSet", "flows", "connection_set", "values", "functions"];

/// # Example
/// ```
/// use std::path::PathBuf;
/// use flowlib::loader::loader;
///
/// let path = PathBuf::from("../samples/hello-world-simple-toml/context.toml");
/// loader::load(path);
/// ```
pub fn load(file_path: PathBuf) -> Result {
    // TODO load a loader object based on extension, then invoke via Trait

    let result = match file_path.extension() {
        Some(ext) => {
            match ext.to_str() {
                Some("yaml") => FlowYamlLoader::load(&file_path),
                Some("toml") => FlowTomelLoader::load(&file_path),
                _    => Result::Error(format!("Unsupported file extension '{:?}'", ext)),
            }
        },
        None => Result::Error("No file extension so cannot determine file format".to_string())
    };

    match result {
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