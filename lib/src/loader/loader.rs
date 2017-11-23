use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::io::prelude::*;
use std::collections::HashMap;

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
    Helper method to read the content of a file found at 'file_path' into a String result
*/
fn get_contents(file_path: &PathBuf) -> Result<String, String> {
    let file = File::open(file_path).unwrap(); // TODO handle error
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();

    match buf_reader.read_to_string(&mut contents) {
        Ok(_) => Ok(contents),
        Err(e) => Err(format!("{}", e))
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
                        Err(e) => Err(format!("{}", e)),
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
        Ok(flow) => {
            match flow.validate() {
                Ok(_) => load_flow_contents(flow),
                Err(e) => Err(e)
            }
        },
        Err(e) => Err(e)
    }
}

fn load_flow_contents(flow: Flow) -> Result<Flow, String> {
    // Load subflows from FlowRefs
    //    pub flow: Option<Vec<FlowRef>>,
    // pub flows: Vec<Box<Flow>>

    // load functions from function refs

    // Validate all is consistent now it's loaded??
    // flow.verify()

    Ok(flow)
}