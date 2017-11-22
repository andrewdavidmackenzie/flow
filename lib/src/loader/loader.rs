use description::flow::Flow;
use std::result;
use std::path::PathBuf;
use loader::yaml_loader::FlowYamlLoader;
use loader::toml_loader::FlowTomelLoader;

pub trait Validate {
    fn validate(&self) -> Result<(), String>;
}

pub trait Loader {
    fn load(file_path: &PathBuf) -> Result<Flow, String>;
}

/// # Example
/// ```
/// use std::path::PathBuf;
/// use flowlib::loader::loader;
///
/// let path = PathBuf::from("../samples/hello-world-simple-toml/context.toml");
/// loader::load(path).unwrap();
/// ```
pub fn load(file_path: PathBuf) -> Result<Flow, String> {
    // TODO load a loader object based on extension, then invoke via Trait

    let result = match file_path.extension() {
        Some(ext) => {
            match ext.to_str() {
                Some("yaml") => FlowYamlLoader::load(&file_path),
                Some("toml") => FlowTomelLoader::load(&file_path),
                _ => Err(format!("Unsupported file extension '{:?}'", ext)),
            }
        }
        None => Err("No file extension so cannot determine file format".to_string())
    };

    // TODO a try or expect here????
    match result {
        Ok(flow) => {
            match flow.validate() {
                Ok(_) => return Ok(flow),
                Err(e) => Err(e)
            }
        },
        Err(e) => Err(e)
    }
}