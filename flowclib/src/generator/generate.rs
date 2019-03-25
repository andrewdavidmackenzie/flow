use std::io::Result;
use std::collections::HashMap;
use std::collections::HashSet;
use model::flow::Flow;
use model::route::Route;
use model::connection::Connection;
use flowrlib::manifest::Manifest;
use flowrlib::process::Process;
use model::function::Function;
use model::name::HasName;
use model::route::HasRoute;

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
            source_routes: HashMap::<Route, (String, usize)>::new(),
            destination_routes: HashMap::<Route, (usize, usize)>::new(),
            collapsed_connections: Vec::new(),
            functions: Vec::new(),
            libs: HashSet::new(),
        }
    }
}

pub fn create_manifest(_flow: &Flow, debug_symbols: bool, out_dir_path: &str, tables: &GenerationTables)
                       -> Result<Manifest> {
    info!("==== Generator: Writing manifest to '{}'", out_dir_path);
    let mut manifest = Manifest::new();
    let mut base_path = out_dir_path.to_string();
    base_path.push('/');

    // Generate runtime Process struct for each of the runnables
    for runnable in &tables.functions {
        manifest.processes.push(runnable_to_process(&base_path, runnable, debug_symbols));
    }

    Ok(manifest)
}

// Do as an Into trait?
fn runnable_to_process(out_dir_path: &str, function: &Box<Function>, debug_symbols: bool) -> Process {
    let mut name = "".to_string();
    let mut route = "".to_string();

    if debug_symbols {
        name = function.alias().to_string();
        route = function.route().to_string();
    }

    let mut implementation_source = function.get_implementation_source();

    // make path to implementation relative to the output directory if under it
    implementation_source = implementation_source.replace(out_dir_path, "");

    let mut process_inputs = vec!();
    match &function.get_inputs() {
        &None => {},
        Some(inputs) => {
            for input in inputs {
                process_inputs.push((input.depth(), input.get_initial_value().clone()));
            }
        }
    };
    let id = function.get_id();
    let output_routes = function.get_output_routes().clone();

    Process::new(name,
                 route,
                 implementation_source,
                 process_inputs,
                 id,
                 output_routes)
}

#[cfg(test)]
mod test {
    use model::io::IO;
    use model::function::Function;
    use super::runnable_to_process;
    use flowrlib::input::{InputInitializer, ConstantInputInitializer};
    use flowrlib::input::OneTimeInputInitializer;

    #[test]
    fn function_with_sub_route_output_to_code() {
        let function = Function::new(
            "Stdout".to_string(),
            false,
            Some("lib://flowr/stdio/stdout".to_string()),
            "print".to_string(),
            Some(vec!()),
            Some(vec!(
                IO::new(&"Json".to_string(), &"".to_string()),
                IO::new(&"String".to_string(), &"".to_string())
            )),
            "file:///fake/file".to_string(),
            "/flow0/stdout".to_string(),
            None,
            vec!(("".to_string(), 1, 0), ("sub_route".to_string(), 2, 0)),
            0);

        let expected = "{
  'id': 0,
  'implementation_source': 'lib://flowr/stdio/stdout',
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

        let process = runnable_to_process("/test", &br, false);

        let serialized_process = serde_json::to_string_pretty(&process).unwrap();
        assert_eq!(serialized_process, expected.replace("'", "\""));
    }

    #[test]
    fn function_to_code() {
        let function = Function::new(
            "Stdout".to_string(),
            false,
            Some("lib://flowr/stdio/stdout".to_string()),
            "print".to_string(),
            Some(vec!()),
            Some(vec!(
                IO::new(&"String".to_string(), &"".to_string())
            )),
            "file:///fake/file".to_string(),
            "/flow0/stdout".to_string(),
            None,
            vec!(("".to_string(), 1, 0)),
            0);

        let expected = "{
  'id': 0,
  'implementation_source': 'lib://flowr/stdio/stdout',
  'output_routes': [
    [
      '',
      1,
      0
    ]
  ]
}";

        let br = Box::new(function) as Box<Function>;

        let process = runnable_to_process("/test", &br, false);

        let serialized_process = serde_json::to_string_pretty(&process).unwrap();
        assert_eq!(serialized_process, expected.replace("'", "\""));
    }

    #[test]
    fn function_with_initialized_input() {
        let mut io = IO::new(&"String".to_string(), &"".to_string());
        io.set_initial_value(&Some(InputInitializer::OneTime(
            OneTimeInputInitializer{ once: json!(1)}
        )));

        let function = Function::new(
            "Stdout".to_string(),
            false,
            Some("lib://flowr/stdio/stdout".to_string()),
            "print".to_string(),
            Some(vec!(io)),
            None,
            "file:///fake/file".to_string(),
            "/flow0/stdout".to_string(),
            None,
            vec!(),
            0);

        let expected = "{
  'id': 0,
  'implementation_source': 'lib://flowr/stdio/stdout',
  'inputs': [
    {
      'initializer': {
        'once': 1
      }
    }
  ]
}";

        let br = Box::new(function) as Box<Function>;
        let process = runnable_to_process("/test", &br, false);

        println!("process {}", process);

        let serialized_process = serde_json::to_string_pretty(&process).unwrap();
        assert_eq!(expected.replace("'", "\""), serialized_process);
    }


    #[test]
    fn function_with_constant_input() {
        let mut io = IO::new(&"String".to_string(), &"".to_string());
        io.set_initial_value(&Some(InputInitializer::Constant(
            ConstantInputInitializer{ constant: json!(1)}
        )));

        let function = Function::new(
            "Stdout".to_string(),
            false,
            Some("lib://flowr/stdio/stdout".to_string()),
            "print".to_string(),
            Some(vec!(io)),
            None,
            "file:///fake/file".to_string(),
            "/flow0/stdout".to_string(),
            None,
            vec!(),
            0);

        let expected = "{
  'id': 0,
  'implementation_source': 'lib://flowr/stdio/stdout',
  'inputs': [
    {
      'initializer': {
        'constant': 1
      }
    }
  ]
}";

        let br = Box::new(function) as Box<Function>;
        let process = runnable_to_process("/test", &br, false);

        println!("process {}", process);

        let serialized_process = serde_json::to_string_pretty(&process).unwrap();
        assert_eq!(expected.replace("'", "\""), serialized_process);
    }

    #[test]
    fn function_to_code_with_debug() {
        let function = Function::new(
            "Stdout".to_string(),
            false,
            Some("lib://flowr/stdio/stdout".to_string()),
            "print".to_string(),
            Some(vec!()),
            Some(vec!(
                IO::new(&"String".to_string(), &"".to_string())
            )),
            "file:///fake/file".to_string(),
            "/flow0/stdout".to_string(),
            None,
            vec!(("".to_string(), 1, 0)),
            0);

        let expected = "{
  'name': 'print',
  'route': '/flow0/stdout',
  'id': 0,
  'implementation_source': 'lib://flowr/stdio/stdout',
  'output_routes': [
    [
      '',
      1,
      0
    ]
  ]
}";

        let br = Box::new(function) as Box<Function>;

        let process = runnable_to_process("/test", &br, true);

        let serialized_process = serde_json::to_string_pretty(&process).unwrap();
        assert_eq!(serialized_process, expected.replace("'", "\""));
    }

    #[test]
    fn function_with_array_element_output() {
        let function = Function::new(
            "Stdout".to_string(),
            false,
            Some("lib://flowr/stdio/stdout".to_string()),
            "print".to_string(),
            Some(vec!()),
            Some(vec!(
                IO::new(&"Array".to_string(), &"".to_string())
            )),
            "file:///fake/file".to_string(),
            "/flow0/stdout".to_string(),
            None,
            vec!(("0".to_string(), 1, 0)),
            0);

        let expected = "{
  'id': 0,
  'implementation_source': 'lib://flowr/stdio/stdout',
  'output_routes': [
    [
      '0',
      1,
      0
    ]
  ]
}";

        let br = Box::new(function) as Box<Function>;

        let process = runnable_to_process("/test", &br, false);

        let serialized_process = serde_json::to_string_pretty(&process).unwrap();
        assert_eq!(serialized_process, expected.replace("'", "\""));
    }
}