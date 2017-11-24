use std::fs::File;
use std::fs;
use std::io::BufReader;
use std::path::PathBuf;
use std::io::prelude::*;
use std::collections::HashMap;
use std::io;

use description::flow::Flow;
use loader::yaml_loader::FlowYamlLoader;
use loader::toml_loader::FlowTomelLoader;

pub trait Validate {
    fn validate(&self) -> Result<(), String>;
}

pub trait Loader {
    fn load_flow(&self, contents: &str) -> Result<Flow, String>;
}

/*
    Helper method to read the content of a file found at 'file_path' into a String result.
    'file_path' could be absolute or relative, so we canonicalize it first...
*/
fn get_contents(file_path: &PathBuf) -> Result<String, String> {
    match File::open(file_path) {
        Ok(file) => {
            let mut buf_reader = BufReader::new(file);
            let mut contents = String::new();

            match buf_reader.read_to_string(&mut contents) {
                Ok(_) => Ok(contents),
                Err(e) => Err(format!("{}", e))
            }
        },
        Err(e) => Err(format!("{}", e))
    }
}

#[test]
fn get_contents_file_not_found() {
    match get_contents(&PathBuf::from("no-such-file")) {
        Ok(_) => assert!(false),
        Err(e) => {}
    }
}

#[test]
fn get_contents_file_found() {
    match get_contents(&PathBuf::from(file!())) {
        Ok(_) => {},
        Err(e) => assert!(false)
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
    let mut loaders = HashMap::new();
    loaders.insert("toml", &FlowTomelLoader{} as &Loader);
    loaders.insert("yaml", &FlowYamlLoader{} as &Loader);

    let result = match file_path.extension() {
        Some(ext) => {
            /*
            match loaders.get(ext) {
                Some(&loader) => {
                */
            let toml = &FlowTomelLoader{};

            match get_contents(&file_path) {
                        Ok(contents) => toml.load_flow(&contents),
                        Err(e) => Err(e),
                    }
            /*
                },
                _ => Err(format!("No loader found for file extension '{:?}'", ext)),
            }
            */
        },
        None => Err("No file extension so cannot determine file format".to_string())
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
                },
                Err(e) => Err(e)
            }
        },
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

    //    pub flow: Option<Vec<FlowRef>>,
    // pub flows: Vec<Box<Flow>>

    // load functions from function refs

    // Validate all is consistent now it's loaded??
    // flow.verify()

    Ok(())
}