use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use log::info;
use serde_derive::Serialize;
use url::Url;

use flowcore::function::Function as RuntimeFunction;
use flowcore::input::Input;
use flowcore::manifest::{Manifest, MetaData, DEFAULT_MANIFEST_FILENAME};

use crate::errors::*;
use crate::model::connection::Connection;
use crate::model::flow::Flow;
use crate::model::function::Function;
use crate::model::io::IO;
use crate::model::name::HasName;
#[cfg(feature = "debugger")]
use crate::model::route::HasRoute;
use crate::model::route::Route;

#[derive(Serialize, Default)]
pub struct GenerationTables {
    pub connections: Vec<Connection>,
    pub source_routes: HashMap<Route, (Route, usize)>,
    /// HashMap from "route of the output of a function" --> (output name, source_function_id)
    pub destination_routes: HashMap<Route, (usize, usize, usize)>,
    /// HashMap from "route of the input of a function" --> (destination_function_id, input number, flow_id)
    pub collapsed_connections: Vec<Connection>,
    pub functions: Vec<Function>,
    pub libs: HashSet<Url>,
    pub source_files: Vec<String>,
}

impl GenerationTables {
    pub fn new() -> Self {
        GenerationTables {
            connections: Vec::new(),
            source_routes: HashMap::<Route, (Route, usize)>::new(),
            destination_routes: HashMap::<Route, (usize, usize, usize)>::new(),
            collapsed_connections: Vec::new(),
            functions: Vec::new(),
            libs: HashSet::new(),
            source_files: Vec::new(),
        }
    }
}

impl From<&Flow> for MetaData {
    fn from(flow: &Flow) -> Self {
        flow.metadata.clone()
    }
}

impl From<&IO> for Input {
    fn from(io: &IO) -> Self {
        Input::new(io.get_initializer())
    }
}

/*
    Paths in the manifest are relative to the location of the manifest file, to make the file
    and associated files relocatable (and maybe packaged into a ZIP etc). So we use manifest_url
    as the location other file paths are made relative to.
*/
pub fn create_manifest(
    flow: &Flow,
    debug_symbols: bool,
    manifest_url: &Url,
    tables: &GenerationTables,
    source_urls: HashSet<(Url, Url)>,
) -> Result<Manifest> {
    info!("Writing flow manifest to '{}'", manifest_url);

    let mut manifest = Manifest::new(MetaData::from(flow));

    // Generate run-time Function struct for each of the compile-time functions
    for function in &tables.functions {
        manifest.add_function(function_to_runtimefunction(
            manifest_url,
            function,
            debug_symbols,
        )?);
    }

    manifest.set_lib_references(&tables.libs);
    manifest.set_source_urls(source_urls);

    Ok(manifest)
}

/// Generate a manifest for the flow in JSON that can be used to run it using 'flowr'
// TODO this is tied to being a file:// - generalize this to write to a URL, moving the code
// TODO into the provider and implementing for file and http
pub fn write_flow_manifest(
    flow: Flow,
    debug_symbols: bool,
    destination: &Path,
    tables: &GenerationTables,
    source_urls: HashSet<(Url, Url)>,
) -> Result<PathBuf> {
    let mut filename = destination.to_path_buf();
    filename.push(DEFAULT_MANIFEST_FILENAME.to_string());
    filename.set_extension("json");
    let mut manifest_file =
        File::create(&filename).chain_err(|| "Could not create manifest file")?;
    let manifest_url =
        Url::from_file_path(&filename).map_err(|_| "Could not parse Url from file path")?;
    let manifest = create_manifest(&flow, debug_symbols, &manifest_url, tables, source_urls)
        .chain_err(|| "Could not create manifest from parsed flow and compiler tables")?;

    manifest_file
        .write_all(
            serde_json::to_string_pretty(&manifest)
                .chain_err(|| "Could not pretty format the manifest JSON contents")?
                .as_bytes(),
        )
        .chain_err(|| "Could not write manifest data bytes to created manifest file")?;

    Ok(filename)
}

/*
    Create a run-time function struct from a compile-time function struct.
    manifest_dir is the directory that paths will be made relative to.
*/
fn function_to_runtimefunction(
    manifest_url: &Url,
    function: &Function,
    debug_symbols: bool,
) -> Result<RuntimeFunction> {
    #[cfg(feature = "debugger")]
    let name = if debug_symbols {
        function.alias().to_string()
    } else {
        "".to_string()
    };

    #[cfg(feature = "debugger")]
    let route = if debug_symbols {
        function.route().to_string()
    } else {
        "".to_string()
    };

    // make the location of implementation relative to the output directory if it is under it
    let implementation_location = implementation_location_relative(&function, manifest_url)?;

    let mut runtime_inputs = vec![];
    for input in function.get_inputs() {
        runtime_inputs.push(Input::from(input));
    }

    Ok(RuntimeFunction::new(
        #[cfg(feature = "debugger")]
        name,
        #[cfg(feature = "debugger")]
        route,
        implementation_location,
        runtime_inputs,
        function.get_id(),
        function.get_flow_id(),
        function.get_output_connections(),
        debug_symbols,
    ))
}

/*
    Get the location of the implementation - relative to the Manifest if it is a provided implementation
*/
// TODO generalize this for Urls, not just files - will require changing the function.get_implementation()
fn implementation_location_relative(function: &Function, manifest_url: &Url) -> Result<String> {
    if let Some(ref lib_reference) = function.get_lib_reference() {
        Ok(format!("lib://{}/{}", lib_reference, &function.name()))
    } else {
        let implementation_path = function.get_implementation();
        let implementation_url = Url::from_file_path(implementation_path)
            .map_err(|_| {
                format!(
                    "Could not create Url from file path: {}",
                    implementation_path
                )
            })?
            .to_string();

        let mut manifest_base_url = manifest_url.clone();
        manifest_base_url
            .path_segments_mut()
            .map_err(|_| "cannot be base")?
            .pop();

        info!("Manifest base = '{}'", manifest_base_url.to_string());
        info!("Absolute implementation path = '{}'", implementation_path);
        let relative_path =
            implementation_url.replace(&format!("{}/", manifest_base_url.as_str()), "");
        info!("Relative implementation path = '{}'", relative_path);
        Ok(relative_path)
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use url::Url;

    use flowcore::input::InputInitializer;
    use flowcore::output_connection::OutputConnection;

    use crate::model::function::Function;
    use crate::model::io::IO;
    use crate::model::name::Name;
    use crate::model::route::Route;

    use super::function_to_runtimefunction;

    #[test]
    fn function_with_sub_route_output_generation() {
        let function = Function::new(
            Name::from("Stdout"),
            false,
            "lib://flowruntime/stdio/stdout".to_string(),
            Name::from("print"),
            vec![],
            vec![
                IO::new("Value", Route::default()),
                IO::new("String", Route::default()),
            ],
            "file:///fake/file",
            Route::from("/flow0/stdout"),
            Some("flowruntime/stdio/stdout".to_string()),
            vec![
                OutputConnection::new(
                    "".to_string(),
                    1,
                    0,
                    0,
                    0,
                    false,
                    String::default(),
                    #[cfg(feature = "debugger")]
                    String::default(),
                ),
                OutputConnection::new(
                    "sub_route".to_string(),
                    2,
                    0,
                    0,
                    0,
                    false,
                    String::default(),
                    #[cfg(feature = "debugger")]
                    String::default(),
                ),
            ],
            0,
            0,
        );

        let expected = "{
  'id': 0,
  'flow_id': 0,
  'implementation_location': 'lib://flowruntime/stdio/stdout/Stdout',
  'output_connections': [
    {
      'function_id': 1,
      'io_number': 0,
      'flow_id': 0
    },
    {
      'subroute': 'sub_route',
      'function_id': 2,
      'io_number': 0,
      'flow_id': 0
    }
  ]
}";

        let br = Box::new(function) as Box<Function>;

        let runtime_process = function_to_runtimefunction(
            &Url::parse("file://test").expect("Couldn't parse test Url"),
            &br,
            false,
        )
        .expect("Could not convert compile time function to runtime function");

        let serialized_process = serde_json::to_string_pretty(&runtime_process)
            .expect("Could not convert function content to json");
        assert_eq!(serialized_process, expected.replace("'", "\""));
    }

    #[test]
    fn function_generation() {
        let function = Function::new(
            Name::from("Stdout"),
            false,
            "lib://flowruntime/stdio/stdout".to_string(),
            Name::from("print"),
            vec![],
            vec![IO::new("String", Route::default())],
            "file:///fake/file",
            Route::from("/flow0/stdout"),
            Some("flowruntime/stdio/stdout".to_string()),
            vec![OutputConnection::new(
                "".to_string(),
                1,
                0,
                0,
                0,
                false,
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
            )],
            0,
            0,
        );

        let expected = "{
  'id': 0,
  'flow_id': 0,
  'implementation_location': 'lib://flowruntime/stdio/stdout/Stdout',
  'output_connections': [
    {
      'function_id': 1,
      'io_number': 0,
      'flow_id': 0
    }
  ]
}";

        let br = Box::new(function) as Box<Function>;

        let process = function_to_runtimefunction(
            &Url::parse("file://test").expect("Couldn't parse test Url"),
            &br,
            false,
        )
        .expect("Could not convert compile time function to runtime function");

        let serialized_process = serde_json::to_string_pretty(&process)
            .expect("Could not convert function content to json");
        assert_eq!(serialized_process, expected.replace("'", "\""));
    }

    #[test]
    fn function_generation_with_array_order() {
        let function = Function::new(
            Name::from("Stdout"),
            false,
            "lib://flowruntime/stdio/stdout".to_string(),
            Name::from("print"),
            vec![],
            vec![IO::new("String", Route::default())],
            "file:///fake/file",
            Route::from("/flow0/stdout"),
            Some("flowruntime/stdio/stdout".to_string()),
            vec![OutputConnection::new(
                "".to_string(),
                1,
                0,
                0,
                1,
                false,
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
            )],
            0,
            0,
        );

        let expected = "{
  'id': 0,
  'flow_id': 0,
  'implementation_location': 'lib://flowruntime/stdio/stdout/Stdout',
  'output_connections': [
    {
      'function_id': 1,
      'io_number': 0,
      'flow_id': 0,
      'array_level_serde': 1
    }
  ]
}";

        let br = Box::new(function) as Box<Function>;

        let process = function_to_runtimefunction(
            &Url::parse("file://test").expect("Couldn't parse test Url"),
            &br,
            false,
        )
        .expect("Could not convert compile time function to runtime function");

        let serialized_process = serde_json::to_string_pretty(&process)
            .expect("Could not convert function content to json");
        assert_eq!(serialized_process, expected.replace("'", "\""));
    }

    #[test]
    fn function_with_initialized_input_generation() {
        let mut io = IO::new("String", Route::default());
        io.set_initializer(&Some(InputInitializer::Once(json!(1))));

        let function = Function::new(
            Name::from("Stdout"),
            false,
            "lib://flowruntime/stdio/stdout".to_string(),
            Name::from("print"),
            vec![io],
            vec![],
            "file:///fake/file",
            Route::from("/flow0/stdout"),
            Some("flowruntime/stdio/stdout".to_string()),
            vec![],
            0,
            0,
        );

        let expected = "{
  'id': 0,
  'flow_id': 0,
  'implementation_location': 'lib://flowruntime/stdio/stdout/Stdout',
  'inputs': [
    {
      'initializer': {
        'once': 1
      }
    }
  ]
}";

        let br = Box::new(function) as Box<Function>;
        let process = function_to_runtimefunction(
            &Url::parse("file://test").expect("Couldn't parse test Url"),
            &br,
            false,
        )
        .expect("Could not convert compile time function to runtime function");

        let serialized_process = serde_json::to_string_pretty(&process)
            .expect("Could not convert function content to json");
        assert_eq!(expected.replace("'", "\""), serialized_process);
    }

    #[test]
    fn function_with_constant_input_generation() {
        let mut io = IO::new("String", Route::default());
        io.set_initializer(&Some(InputInitializer::Always(json!(1))));

        let function = Function::new(
            Name::from("Stdout"),
            false,
            "lib://flowruntime/stdio/stdout".to_string(),
            Name::from("print"),
            vec![io],
            vec![],
            "file:///fake/file",
            Route::from("/flow0/stdout"),
            Some("flowruntime/stdio/stdout".to_string()),
            vec![],
            0,
            0,
        );

        let expected = "{
  'id': 0,
  'flow_id': 0,
  'implementation_location': 'lib://flowruntime/stdio/stdout/Stdout',
  'inputs': [
    {
      'initializer': {
        'always': 1
      }
    }
  ]
}";

        let br = Box::new(function) as Box<Function>;
        let process = function_to_runtimefunction(
            &Url::parse("file://test").expect("Couldn't parse test Url"),
            &br,
            false,
        )
        .expect("Could not convert compile time function to runtime function");

        let serialized_process = serde_json::to_string_pretty(&process)
            .expect("Could not convert function content to json");
        assert_eq!(expected.replace("'", "\""), serialized_process);
    }

    #[test]
    fn function_with_array_input_generation() {
        let io = IO::new("Array/String", Route::default());

        let function = Function::new(
            Name::from("Stdout"),
            false,
            "lib://flowruntime/stdio/stdout".to_string(),
            Name::from("print"),
            vec![io],
            vec![],
            "file:///fake/file",
            Route::from("/flow0/stdout"),
            Some("flowruntime/stdio/stdout".to_string()),
            vec![],
            0,
            0,
        );

        let expected = "{
  'id': 0,
  'flow_id': 0,
  'implementation_location': 'lib://flowruntime/stdio/stdout/Stdout',
  'inputs': [
    {}
  ]
}";

        let br = Box::new(function) as Box<Function>;
        let process = function_to_runtimefunction(
            &Url::parse("file://test").expect("Couldn't parse test Url"),
            &br,
            false,
        )
        .expect("Could not convert compile time function to runtime function");

        let serialized_process = serde_json::to_string_pretty(&process)
            .expect("Could not convert function content to json");
        assert_eq!(serialized_process, expected.replace("'", "\""));
    }

    fn test_function() -> Function {
        Function::new(
            Name::from("Stdout"),
            false,
            "lib://flowruntime/stdio/stdout".to_string(),
            Name::from("print"),
            vec![],
            vec![IO::new("String", Route::default())],
            "file:///fake/file",
            Route::from("/flow0/stdout"),
            Some("flowruntime/stdio/stdout".to_string()),
            vec![OutputConnection::new(
                "".to_string(),
                1,
                0,
                0,
                0,
                false,
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
            )],
            0,
            0,
        )
    }

    #[test]
    fn function_to_code_with_debug_generation() {
        let function = test_function();

        #[cfg(feature = "debugger")]
        let expected = "{
  'name': 'print',
  'route': '/flow0/stdout',
  'id': 0,
  'flow_id': 0,
  'implementation_location': 'lib://flowruntime/stdio/stdout/Stdout',
  'output_connections': [
    {
      'function_id': 1,
      'io_number': 0,
      'flow_id': 0
    }
  ]
}";
        #[cfg(not(feature = "debugger"))]
        let expected = "{
  'id': 0,
  'flow_id': 0,
  'implementation_location': 'lib://flowruntime/stdio/stdout/Stdout',
  'output_connections': [
    {
      'function_id': 1,
      'io_number': 0,
      'flow_id': 0
    }
  ]
}";
        let br = Box::new(function) as Box<Function>;

        let process = function_to_runtimefunction(
            &Url::parse("file://test").expect("Couldn't parse test Url"),
            &br,
            true,
        )
        .expect("Could not convert compile time function to runtime function");

        let serialized_process = serde_json::to_string_pretty(&process)
            .expect("Could not convert function content to json");
        assert_eq!(serialized_process, expected.replace("'", "\""));
    }

    #[test]
    fn function_with_array_element_output_generation() {
        let function = Function::new(
            Name::from("Stdout"),
            false,
            "lib://flowruntime/stdio/stdout".to_string(),
            Name::from("print"),
            vec![],
            vec![IO::new("Array", Route::default())],
            "file:///fake/file",
            Route::from("/flow0/stdout"),
            Some("flowruntime/stdio/stdout".to_string()),
            vec![OutputConnection::new(
                "/0".to_string(),
                1,
                0,
                0,
                0,
                false,
                String::default(),
                #[cfg(feature = "debugger")]
                String::default(),
            )],
            0,
            0,
        );

        let expected = "{
  'id': 0,
  'flow_id': 0,
  'implementation_location': 'lib://flowruntime/stdio/stdout/Stdout',
  'output_connections': [
    {
      'subroute': '/0',
      'function_id': 1,
      'io_number': 0,
      'flow_id': 0
    }
  ]
}";

        let br = Box::new(function) as Box<Function>;

        let process = function_to_runtimefunction(
            &Url::parse("file://test").expect("Couldn't parse test Url"),
            &br,
            false,
        )
        .expect("Could not convert compile time function to runtime function");

        let serialized_process = serde_json::to_string_pretty(&process)
            .expect("Could not convert function content to json");
        assert_eq!(serialized_process, expected.replace("'", "\""));
    }
}
