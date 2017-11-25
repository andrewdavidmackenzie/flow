use std::fs::File;
use std::fs;
use std::io::BufReader;
use std::path::PathBuf;
use std::io::prelude::*;
use std::fmt;
use std::env;

use description::flow::Flow;
use description::function::Function;
use loader::yaml_loader::FlowYamlLoader;
use loader::toml_loader::FlowTomelLoader;
use description::name::Name;
use description::name::Named;

pub trait Loader {
    fn load_flow(&self, contents: &str) -> Result<Flow, String>;
    fn load_function(&self, contents: &str) -> Result<Function, String>;
}

pub trait Validate {
    fn validate(&self) -> Result<(), String>;
}

#[derive(Deserialize, Debug)]
pub struct Reference {
    pub name: Name,
    pub source: String
}

// TODO figure out how to have this derived automatically for types needing it
impl Named for Reference {
    fn name(&self) -> &str {
        &self.name[..]
    }
}

impl Validate for Reference {
    fn validate(&self) -> Result<(), String> {
        self.name.validate()
        // Pretty much anything is a valid PathBuf - so not sure how to validate source...
    }
}

impl fmt::Display for Reference {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Reference:\n\tname: {}\n\tsource: {}", self.name, self.source)
    }
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

#[test]
#[should_panic]
fn no_extension() {
    get_loader(&PathBuf::from("no_extension")).unwrap();
}

#[test]
#[should_panic]
fn invalid_extension() {
    get_loader(&PathBuf::from("no_extension.wrong")).unwrap();
}

#[test]
fn valid_extension() {
    get_loader(&PathBuf::from("OK.toml")).unwrap();
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
#[should_panic]
fn get_contents_file_not_found() {
    get_contents(&PathBuf::from("no-such-file")).unwrap();
}

// NOTE: these unwraps fail if the files don't actually exist!
fn get_canonical_path(parent_path: PathBuf, child_path: PathBuf) -> PathBuf {
    if child_path.is_relative() {
        fs::canonicalize(parent_path).unwrap().parent().unwrap().join(child_path)
    } else {
        child_path
    }
}

#[test]
fn absolute_path() {
    let path = get_canonical_path(PathBuf::from("/root/me/original_file"),
                                  PathBuf::from("/users/home/my_file"));
    assert_eq!(path.to_str().unwrap(), "/users/home/my_file");
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
    let loader = get_loader(&file_path)?;
    let contents = get_contents(&file_path)?;
    let mut flow = loader.load_flow(&contents)?;
    flow.source = file_path;
    flow.validate()?;
    load_references(&mut flow)?;
    flow.build_connections()?;
    Ok(flow)
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
    let loader = get_loader(&file_path)?;
    let contents = get_contents(&file_path)?;
    let mut function = loader.load_function(&contents)?;
    function.source = file_path;
    function.validate()?;
    Ok(function)
}

fn load_references(flow: &mut Flow) -> Result<(), String> {
    // Load subflows from References
    if let Some(ref flow_refs) = flow.flow {
        for ref flow_ref in flow_refs {
            let subflow_path = get_canonical_path(PathBuf::from(&flow.source),
                                                  PathBuf::from(&flow_ref.source));
            let subflow = load_flow(subflow_path)?;
            flow.flows.push(subflow);
        }
    }

    // load functions from References
    if let Some(ref function_refs) = flow.function {
        for ref function_ref in function_refs {
            let function_path = get_canonical_path(PathBuf::from(&flow.source),
                                                   PathBuf::from(&function_ref.source));
            let function = load_function(function_path)?;
            flow.functions.push(function);
        }
    }

    Ok(())
}