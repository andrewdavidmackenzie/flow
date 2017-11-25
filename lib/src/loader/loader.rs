use std::fs::File;
use std::fs;
use std::io::BufReader;
use std::path::PathBuf;
use std::io::prelude::*;
use description::flow::Flow;
use description::function::Function;
use loader::yaml_loader::FlowYamlLoader;
use loader::toml_loader::FlowTomelLoader;

pub trait Loader {
    fn load_flow(&self, contents: &str) -> Result<Flow, String>;
    fn load_function(&self, contents: &str) -> Result<Function, String>;
}

pub trait Validate {
    fn validate(&self) -> Result<(), String>;
}

const TOML: &Loader = &FlowTomelLoader {} as &Loader;
const YAML: &Loader = &FlowYamlLoader {} as &Loader;

fn get_loader(file_path: &PathBuf) -> Result<&'static Loader, String> {
    match file_path.extension() {
        Some(ext) => {
            match ext.to_str() {
                Some("toml") => Ok(TOML),
                Some("yaml") => Ok(YAML),
                _ => Err("Unknown file extension so cannot determine loader to use".to_string())
            }
        }
        None => Err("No file extension so cannot determine loader to use".to_string())
    }
}

// Helper method to read the content of a file found at 'file_path' into a String result.
// 'file_path' could be absolute or relative, so we canonicalize it first...
fn get_contents(file_path: &PathBuf) -> Result<String, String> {
    match File::open(file_path) {
        Ok(file) => {
            let mut buf_reader = BufReader::new(file);
            let mut contents = String::new();

            match buf_reader.read_to_string(&mut contents) {
                Ok(_) => Ok(contents),
                Err(e) => Err(format!("{}", e))
            }
        }
        Err(e) => Err(format!("{}", e))
    }
}

#[test]
fn get_contents_file_not_found() {
    match get_contents(&PathBuf::from("no-such-file")) {
        Ok(_) => assert!(false),
        Err(_) => {}
    }
}

/// # Example
/// ```
/// use std::path::PathBuf;
/// use flowlib::loader::loader;
///
/// let path = PathBuf::from("../samples/hello-world-simple-toml/context.toml");
/// loader::load_flow(path).unwrap();
/// ```
pub fn load_flow(file_path: PathBuf) -> Result<Flow, String> {
    let loader = get_loader(&file_path).unwrap();

    let result = match get_contents(&file_path) {
        Ok(contents) => loader.load_flow(&contents),
        Err(e) => Err(e),
    };

    // TODO a try or expect here????
    match result {
        Ok(mut flow) => {
            match flow.validate() {
                Ok(_) => {
                    flow.source = file_path;
                    match load_flow_contents(&mut flow) {
                        Ok(_) => Ok(flow),
                        Err(e) => Err(e)
                    }
                }
                Err(e) => Err(e)
            }
        }
        Err(e) => Err(e)
    }
}

/// # Example
/// ```
/// use std::path::PathBuf;
/// use flowlib::loader::loader;
///
/// let path = PathBuf::from("../samples/hello-world-simple-toml/terminal.toml");
/// loader::load_function(path).unwrap();
/// ```
pub fn load_function(file_path: PathBuf) -> Result<Function, String> {
    let loader = get_loader(&file_path).unwrap();

    let result = match get_contents(&file_path) {
        Ok(contents) => loader.load_function(&contents),
        Err(e) => Err(e),
    };

    // TODO a try or expect here????
    match result {
        Ok(mut function) => {
            function.source = file_path;
            function.validate()?;
            Ok(function)
        }
        Err(e) => Err(e)
    }
}

fn get_canonical_path(parent_path: PathBuf, child_path: PathBuf) -> PathBuf {
    if child_path.is_relative() {
        fs::canonicalize(parent_path).unwrap().parent().unwrap().join(child_path)
    } else {
        child_path
    }
}

fn load_flow_contents(flow: &mut Flow) -> Result<(), String> {
    // Load subflows from FlowRefs
    if let Some(ref flow_refs) = flow.flow {
        for ref flow_ref in flow_refs {
            let subflow_path = get_canonical_path(PathBuf::from(&flow.source),
                                                  PathBuf::from(&flow_ref.source));
            let subflow = load_flow(subflow_path)?;
            flow.flows.push(subflow);
        }
    }

    // load functions from function refs
    if let Some(ref function_refs) = flow.function {
        for ref function_ref in function_refs {
            let function_path = get_canonical_path(PathBuf::from(&flow.source),
                                                   PathBuf::from(&function_ref.source));
            let function = load_function(function_path)?;
            flow.functions.push(function);
        }
    }

    // verify all is consistent now it's loaded
    flow.verify()
}