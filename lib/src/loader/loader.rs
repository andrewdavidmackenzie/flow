use std::path::PathBuf;

use model::flow::Flow;
use model::function::Function;
use loader::file_helper::get_contents;
use loader::file_helper::get_canonical_path;
use loader::loader_helper::get_loader;

// Any loader of a file extension have to implement these methods
pub trait Loader {
    fn load_flow(&self, contents: &str) -> Result<Flow, String>;
    fn load_function(&self, contents: &str) -> Result<Function, String>;
}

pub trait Validate {
    fn validate(&self) -> Result<(), String>;
}

/// # Example
/// ```
/// use std::path::PathBuf;
/// use flowlib::loader::loader;
///
/// let path = PathBuf::from("../samples/hello-world-simple/context.toml");
/// loader::load_flow("", path).unwrap();
/// ```
pub fn load_flow(parent_route: &str, file_path: PathBuf) -> Result<Flow, String> {
    let mut flow = load_single_flow(parent_route, file_path)?;
    load_subflows(&mut flow)?;
    flow.normalize_connection_names();
    Ok(flow)
}

/// # Example
/// ```
/// use std::path::PathBuf;
/// use flowlib::loader::loader;
///
/// let path = PathBuf::from("../samples/hello-world-simple/context.toml");
/// loader::load_single_flow("", path).unwrap();
/// ```
pub fn load_single_flow(parent_route: &str, file_path: PathBuf) -> Result<Flow, String> {
    let loader = get_loader(&file_path)?;
    let contents = get_contents(&file_path)?;
    let mut flow = loader.load_flow(&contents)?;
    flow.source = file_path;
    flow.route = format!("{}/{}", parent_route, flow.name);
    flow.validate()?;
    load_functions(&mut flow)?;
    load_values(&mut flow)?;
    flow.set_io_routes();
    Ok(flow)
}

/// # Example
/// ```
/// use std::path::PathBuf;
/// use flowlib::loader::loader;
///
/// let path = PathBuf::from("../samples/hello-world-simple/terminal.toml");
/// loader::load_function(&path, "").unwrap();
/// ```
pub fn load_function(file_path: &PathBuf, parent_name: &str) -> Result<Function, String> {
    let loader = get_loader(file_path)?;
    let contents = get_contents(file_path)?;
    let mut function = loader.load_function(&contents)?;
    function.route = format!("{}/{}", parent_name, function.name);

    if let Some(ref mut inputs) = function.inputs {
        for ref mut input in inputs {
            input.route = format!("{}/{}", function.route, input.name);
        }
    }
    if let Some(ref mut outputs) = function.outputs {
        for ref mut output in outputs {
            output.route = format!("{}/{}", function.route, output.name);
        }
    }
    function.validate()?;
    Ok(function)
}

/*
    Load all functions referenced from a flow
*/
fn load_functions(flow: &mut Flow) -> Result<(), String> {
    if let Some(ref mut function_refs) = flow.function_refs {
        for ref mut function_ref in function_refs {
            let function_path = get_canonical_path(PathBuf::from(&flow.source),
                                                   PathBuf::from(&function_ref.source));
            function_ref.function = load_function(&function_path, &flow.route)?;
        }
    }
    Ok(())
}

/*
    Load all values defined in a flow
*/
fn load_values(flow: &mut Flow) -> Result<(), String> {
    if let Some(ref mut values) = flow.values {
        for ref mut value in values {
            value.route = format!("{}/{}", flow.route, value.name);
        }
    }
    Ok(())
}

/*
    Load all subflows referenced from a flow
*/
fn load_subflows(flow: &mut Flow) -> Result<(), String> {
    // Load subflows from References
    if let Some(ref mut flow_refs) = flow.flow_refs {
        for ref mut flow_ref in flow_refs {
            let subflow_path = get_canonical_path(PathBuf::from(&flow.source),
                                                  PathBuf::from(&flow_ref.source));
            let subflow = load_flow(&flow.route, subflow_path)?;
            flow_ref.flow = subflow;
        }
    }
    Ok(())
}