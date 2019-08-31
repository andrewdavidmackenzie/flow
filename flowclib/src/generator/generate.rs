use std::collections::HashMap;
use std::collections::HashSet;

use flowrlib::function::Function as RuntimeFunction;
use flowrlib::input::Input;
use flowrlib::manifest::{Manifest, MetaData};

use crate::errors::*;
use crate::model::connection::Connection;
use crate::model::datatype::TypeCheck;
use crate::model::flow::Flow;
use crate::model::function::Function;
use crate::model::io::IO;
use crate::model::name::HasName;
use crate::model::route::HasRoute;
use crate::model::route::Route;

#[derive(Serialize)]
pub struct GenerationTables {
    pub connections: Vec<Connection>,
    pub source_routes: HashMap<Route, (Route, usize)>,
    pub destination_routes: HashMap<Route, (usize, usize)>,
    pub collapsed_connections: Vec<Connection>,
    pub functions: Vec<Box<Function>>,
    pub libs: HashSet<String>,
}

impl GenerationTables {
    pub fn new() -> Self {
        GenerationTables {
            connections: Vec::new(),
            source_routes: HashMap::<Route, (Route, usize)>::new(),
            destination_routes: HashMap::<Route, (usize, usize)>::new(),
            collapsed_connections: Vec::new(),
            functions: Vec::new(),
            libs: HashSet::new(),
        }
    }
}

impl From<&Flow> for MetaData {
    fn from(flow: &Flow) -> Self {
        MetaData {
            alias: flow.alias.clone().to_string(),
            version: flow.version.clone(),
            author_name: flow.author_name.clone(),
            author_email: flow.author_email.clone(),
        }
    }
}

impl From<&IO> for Input {
    fn from(io: &IO) -> Self {
        Input::new(io.depth(), io.get_initializer(), io.datatype(0).is_array())
    }
}

pub fn create_manifest(flow: &Flow, debug_symbols: bool, out_dir: &str, tables: &GenerationTables)
                       -> Result<Manifest> {
    info!("==== Generator: Writing manifest to '{}'", out_dir);

    let mut manifest = Manifest::new(MetaData::from(flow));

    // Generate runtime Process struct for each of the functions
    for function in &tables.functions {
        manifest.add_function(function_to_runtimefunction(&out_dir, function, debug_symbols)?);
    }

    manifest.lib_references = tables.libs.clone();

    Ok(manifest)
}

fn function_to_runtimefunction(out_dir: &str, function: &Box<Function>, debug_symbols: bool) -> Result<RuntimeFunction> {
    let mut name = function.alias().to_string();
    let mut route = function.route().to_string();

    if !debug_symbols {
        name = "".to_string();
        route = "".to_string();
    }

    // make location of implementation relative to the output directory if under it
    let implementation_location = implementation_location_relative(&function, out_dir)?;

    let mut runtime_inputs = vec!();
    match &function.get_inputs() {
        &None => {}
        Some(inputs) => {
            for input in inputs {
                runtime_inputs.push(Input::from(input));
            }
        }
    };

    Ok(RuntimeFunction::new(name,
                            route,
                            implementation_location,
                            function.is_impure(),
                            runtime_inputs,
                            function.get_id(),
                            function.get_output_routes()))
}

/*
    Get the location of the implementation - relative to the Manifest if it is a provided implementation
*/
fn implementation_location_relative(function: &Function, out_dir: &str) -> Result<String> {
    if let Some(ref lib_reference) = function.get_lib_reference() {
        Ok(format!("lib://{}/{}", lib_reference, &function.name()))
    } else {
        match &function.get_implementation() {
            Some(implementation_path) => {
                info!("Out_dir = '{}'", out_dir);
                info!("Absolute implementation path = '{}'", implementation_path);
                let relative_path = implementation_path.replace(out_dir, "");
                info!("Absolute implementation path = '{}'", relative_path);
                Ok(relative_path)
            }
            None => {
                bail!("Function '{}' is not a lib reference but no implementation is provided", function.name())
            }
        }
    }
}

#[cfg(test)]
mod test {
    use flowrlib::input::{ConstantInputInitializer, InputInitializer};
    use flowrlib::input::OneTimeInputInitializer;

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
            Some("lib://runtime/stdio/stdout".to_string()),
            Name::from("print"),
            Some(vec!()),
            Some(vec!(
                IO::new("Json", &Route::default()),
                IO::new("String", &Route::default())
            )),
            "file:///fake/file",
            Route::from("/flow0/stdout"),
            None,
            vec!(("".to_string(), 1, 0), ("sub_route".to_string(), 2, 0)),
            0);

        let expected = "{
  'id': 0,
  'implementation_location': 'lib://runtime/stdio/stdout',
  'output_routes': [
    [
      '',
      1,
      0
    ],
    [
      'sub_route',
      2,
      0
    ]
  ]
}";

        let br = Box::new(function) as Box<Function>;

        let runtime_process = function_to_runtimefunction("/test", &br, false).unwrap();

        let serialized_process = serde_json::to_string_pretty(&runtime_process).unwrap();
        assert_eq!(serialized_process, expected.replace("'", "\""));
    }

    #[test]
    fn function_generation() {
        let function = Function::new(
            Name::from("Stdout"),
            false,
            Some("lib://runtime/stdio/stdout".to_string()),
            Name::from("print"),
            Some(vec!()),
            Some(vec!(IO::new("String", &Route::default()))),
            "file:///fake/file",
            Route::from("/flow0/stdout"),
            None,
            vec!(("".to_string(), 1, 0)),
            0);

        let expected = "{
  'id': 0,
  'implementation_location': 'lib://runtime/stdio/stdout',
  'output_routes': [
    [
      '',
      1,
      0
    ]
  ]
}";

        let br = Box::new(function) as Box<Function>;

        let process = function_to_runtimefunction("/test", &br, false).unwrap();

        let serialized_process = serde_json::to_string_pretty(&process).unwrap();
        assert_eq!(serialized_process, expected.replace("'", "\""));
    }

    #[test]
    fn impure_function_generation() {
        let function = Function::new(
            Name::from("Stdout"),
            true,
            Some("lib://runtime/stdio/stdout".to_string()),
            Name::from("print"),
            Some(vec!()),
            Some(vec!(IO::new("String", &Route::default()))),
            "file:///fake/file",
            Route::from("/flow0/stdout"),
            None,
            vec!(("".to_string(), 1, 0)),
            0);

        let expected = "{
  'id': 0,
  'implementation_location': 'lib://runtime/stdio/stdout',
  'output_routes': [
    [
      '',
      1,
      0
    ]
  ],
  'impure': true
}";

        let br = Box::new(function) as Box<Function>;

        let process = function_to_runtimefunction("/test", &br, false).unwrap();

        let serialized_process = serde_json::to_string_pretty(&process).unwrap();
        assert_eq!(serialized_process, expected.replace("'", "\""));
    }

    #[test]
    fn function_with_initialized_input_generation() {
        let mut io = IO::new("String", &Route::default());
        io.set_initial_value(&Some(InputInitializer::OneTime(
            OneTimeInputInitializer { once: json!(1) }
        )));

        let function = Function::new(
            Name::from("Stdout"),
            false,
            Some("lib://runtime/stdio/stdout".to_string()),
            Name::from("print"),
            Some(vec!(io)),
            None,
            "file:///fake/file",
            Route::from("/flow0/stdout"),
            None,
            vec!(),
            0);

        let expected = "{
  'id': 0,
  'implementation_location': 'lib://runtime/stdio/stdout',
  'inputs': [
    {
      'initializer': {
        'once': 1
      }
    }
  ]
}";

        let br = Box::new(function) as Box<Function>;
        let process = function_to_runtimefunction("/test", &br, false).unwrap();

        println!("process {}", process);

        let serialized_process = serde_json::to_string_pretty(&process).unwrap();
        assert_eq!(expected.replace("'", "\""), serialized_process);
    }

    #[test]
    fn function_with_constant_input_generation() {
        let mut io = IO::new("String", &Route::default());
        io.set_initial_value(&Some(InputInitializer::Constant(
            ConstantInputInitializer { constant: json!(1) }
        )));

        let function = Function::new(
            Name::from("Stdout"),
            false,
            Some("lib://runtime/stdio/stdout".to_string()),
            Name::from("print"),
            Some(vec!(io)),
            None,
            "file:///fake/file",
            Route::from("/flow0/stdout"),
            None,
            vec!(),
            0);

        let expected = "{
  'id': 0,
  'implementation_location': 'lib://runtime/stdio/stdout',
  'inputs': [
    {
      'initializer': {
        'constant': 1
      }
    }
  ]
}";

        let br = Box::new(function) as Box<Function>;
        let process = function_to_runtimefunction("/test", &br, false).unwrap();

        println!("process {}", process);

        let serialized_process = serde_json::to_string_pretty(&process).unwrap();
        assert_eq!(expected.replace("'", "\""), serialized_process);
    }

    #[test]
    fn function_with_array_input_generation() {
        let io = IO::new("Array/String", &Route::default());

        let function = Function::new(
            Name::from("Stdout"),
            false,
            Some("lib://runtime/stdio/stdout".to_string()),
            Name::from("print"),
            Some(vec!(io)),
            None,
            "file:///fake/file",
            Route::from("/flow0/stdout"),
            None,
            vec!(),
            0);

        let expected = "{
  'id': 0,
  'implementation_location': 'lib://runtime/stdio/stdout',
  'inputs': [
    {
      'is_array': true
    }
  ]
}";

        let br = Box::new(function) as Box<Function>;
        let process = function_to_runtimefunction("/test", &br, false).unwrap();

        println!("process {}", process);

        let serialized_process = serde_json::to_string_pretty(&process).unwrap();
        assert_eq!(expected.replace("'", "\""), serialized_process);
    }

    #[test]
    fn function_to_code_with_debug_generation() {
        let function = Function::new(
            Name::from("Stdout"),
            false,
            Some("lib://runtime/stdio/stdout".to_string()),
            Name::from("print"),
            Some(vec!()),
            Some(vec!(
                IO::new("String", &Route::default())
            )),
            "file:///fake/file",
            Route::from("/flow0/stdout"),
            None,
            vec!(("".to_string(), 1, 0)),
            0);

        let expected = "{
  'name': 'print',
  'route': '/flow0/stdout',
  'id': 0,
  'implementation_location': 'lib://runtime/stdio/stdout',
  'output_routes': [
    [
      '',
      1,
      0
    ]
  ]
}";

        let br = Box::new(function) as Box<Function>;

        let process = function_to_runtimefunction("/test", &br, true).unwrap();

        let serialized_process = serde_json::to_string_pretty(&process).unwrap();
        assert_eq!(serialized_process, expected.replace("'", "\""));
    }

    #[test]
    fn function_with_array_element_output_generation() {
        let function = Function::new(
            Name::from("Stdout"),
            false,
            Some("lib://runtime/stdio/stdout".to_string()),
            Name::from("print"),
            Some(vec!()),
            Some(vec!(IO::new("Array", &Route::default()))),
            "file:///fake/file",
            Route::from("/flow0/stdout"),
            None,
            vec!(("0".to_string(), 1, 0)),
            0);

        let expected = "{
  'id': 0,
  'implementation_location': 'lib://runtime/stdio/stdout',
  'output_routes': [
    [
      '0',
      1,
      0
    ]
  ]
}";

        let br = Box::new(function) as Box<Function>;

        let process = function_to_runtimefunction("/test", &br, false).unwrap();

        let serialized_process = serde_json::to_string_pretty(&process).unwrap();
        assert_eq!(serialized_process, expected.replace("'", "\""));
    }
}