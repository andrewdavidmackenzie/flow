use description::flow::Flow;
use std::result;
use std::path::PathBuf;
use loader::yaml_loader::FlowYamlLoader;
use loader::toml_loader::FlowTomelLoader;

pub enum Result  {
    Loaded(Flow),
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
        Result::Loaded(flow) => {
            match flow.validate() {
                Ok(_) => Result::Loaded(flow),
                Err(e) => Result::Error(e)
            }
        },

        Result::Error(string) => {
            return Result::Error(string);
        }
    }
}