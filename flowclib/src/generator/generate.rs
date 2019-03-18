use std::io::Result;
use std::collections::HashMap;
use std::collections::HashSet;
use model::runnable::Runnable;
use model::flow::Flow;
use model::route::Route;
use model::connection::Connection;
use flowrlib::manifest::Manifest;
use flowrlib::process::Process;

#[derive(Serialize)]
pub struct GenerationTables {
    pub connections: Vec<Connection>,
    pub source_routes: HashMap<Route, (Route, usize)>,
    pub destination_routes: HashMap<Route, (usize, usize)>,
    pub collapsed_connections: Vec<Connection>,
    pub runnables: Vec<Box<Runnable>>,
    pub libs: HashSet<String>,
}

serialize_trait_object!(Runnable);

impl GenerationTables {
    pub fn new() -> Self {
        GenerationTables {
            connections: Vec::new(),
            source_routes: HashMap::<Route, (String, usize)>::new(),
            destination_routes: HashMap::<Route, (usize, usize)>::new(),
            collapsed_connections: Vec::new(),
            runnables: Vec::new(),
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
    for runnable in &tables.runnables {
        manifest.processes.push(runnable_to_process(&base_path, runnable, debug_symbols));
    }

    Ok(manifest)
}

// Do as an Into trait?
fn runnable_to_process(out_dir_path: &str, runnable: &Box<Runnable>, debug_symbols: bool) -> Process {
    let mut name = "".to_string();
    let mut route = "".to_string();

    if debug_symbols {
        name = runnable.alias().to_string();
        route = runnable.route().to_string();
    }

    let is_static = runnable.is_static_value();
    let mut implementation_source = runnable.get_implementation_source();

    // make path to implementation relative to the output directory if under it
    implementation_source = implementation_source.replace(out_dir_path, "");

    let input_depths = match &runnable.get_inputs() {
        &None => vec!(),
        Some(inputs) => {
            let mut depths = vec!();
            for input in inputs {
                depths.push(input.depth());
            }
            depths
        }
    };
    let id = runnable.get_id();
    let initial_value = runnable.get_initial_value();
    let output_routes = runnable.get_output_routes().clone();

    Process::new(name,
                 route,
                 is_static,
                 implementation_source,
                 input_depths,
                 id,
                 initial_value,
                 output_routes)
}

#[cfg(test)]
mod test {
    use serde_json::Value as JsonValue;
    use model::value::Value;
    use model::io::IO;
    use model::function::Function;
    use model::runnable::Runnable;
    use super::runnable_to_process;

    #[test]
    fn test_value_to_code() {
        let value = Value::new("value".to_string(),
                               "String".to_string(),
                               Some(JsonValue::String("Hello-World".to_string())),
                               false,
                               "/flow0/value".to_string(),
                               Some(vec!(IO::new(&"Json".to_string(), &"".to_string()))),
                               vec!(("".to_string(), 1, 0)),
                               1);
        let expected = "{
  'id': 1,
  'implementation_source': 'lib://flowstdlib/zero_fifo/Fifo',
  'initial_value': 'Hello-World',
  'inputs': [
    {}
  ],
  'output_routes': [
    [
      '',
      1,
      0
    ]
  ]
}";
        let br = Box::new(value) as Box<Runnable>;
        let process = runnable_to_process("/test", &br, false);

        let serialized_process = serde_json::to_string_pretty(&process).unwrap();

        assert_eq!(serialized_process, expected.replace("'", "\""));
    }

    #[test]
    fn test_value_to_code_with_debug() {
        let value = Value::new("value".to_string(),
                               "String".to_string(),
                               Some(JsonValue::String("Hello-World".to_string())),
                               false,
                               "/flow0/value".to_string(),
                               Some(vec!(IO::new(&"Json".to_string(), &"".to_string()))),
                               vec!(("".to_string(), 1, 0)),
                               1);
        let expected = "{
  'name': 'value',
  'route': '/flow0/value',
  'id': 1,
  'implementation_source': 'lib://flowstdlib/zero_fifo/Fifo',
  'initial_value': 'Hello-World',
  'inputs': [
    {}
  ],
  'output_routes': [
    [
      '',
      1,
      0
    ]
  ]
}";
        let br = Box::new(value) as Box<Runnable>;
        let process = runnable_to_process("/test", &br, true);

        let serialized_process = serde_json::to_string_pretty(&process).unwrap();

        assert_eq!(serialized_process, expected.replace("'", "\""));
    }

    #[test]
    fn test_constant_value_to_code() {
        let value = Value::new(
            "value".to_string(),
            "String".to_string(),
            Some(JsonValue::String("Hello-World".to_string())),
            true,
            "/flow0/value".to_string(),
            Some(vec!(IO::new(&"Json".to_string(), &"".to_string()))),
            vec!(("".to_string(), 1, 0)),
            1);

        let expected = "{
  'id': 1,
  'implementation_source': 'lib://flowstdlib/zero_fifo/Fifo',
  'is_static': true,
  'initial_value': 'Hello-World',
  'inputs': [
    {}
  ],
  'output_routes': [
    [
      '',
      1,
      0
    ]
  ]
}";

        let br = Box::new(value) as Box<Runnable>;
        let process = runnable_to_process("/test", &br, false);

        let serialized_process = serde_json::to_string_pretty(&process).unwrap();
        assert_eq!(expected.replace("'", "\""), serialized_process);
    }

    #[test]
    fn value_with_sub_route_output_to_code() {
        let value = Value::new(
            "value".to_string(),
            "String".to_string(),
            Some(JsonValue::String("Hello-World".to_string())),
            false,
            "/flow0/value".to_string(),
            Some(vec!(
                IO::new(&"Json".to_string(), &"".to_string()),
                IO::new(&"String".to_string(), &"".to_string()))),
            vec!(("".to_string(), 1, 0), ("sub_route".to_string(), 2, 0)),
            1);

        let expected = "{
  'id': 1,
  'implementation_source': 'lib://flowstdlib/zero_fifo/Fifo',
  'initial_value': 'Hello-World',
  'inputs': [
    {}
  ],
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

        let br = Box::new(value) as Box<Runnable>;
        let process = runnable_to_process("/test", &br, false);

        let serialized_process = serde_json::to_string_pretty(&process).unwrap();
        assert_eq!(serialized_process, expected.replace("'", "\""));
    }

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

        let br = Box::new(function) as Box<Runnable>;

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

        let br = Box::new(function) as Box<Runnable>;

        let process = runnable_to_process("/test", &br, false);

        let serialized_process = serde_json::to_string_pretty(&process).unwrap();
        assert_eq!(serialized_process, expected.replace("'", "\""));
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

        let br = Box::new(function) as Box<Runnable>;

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

        let br = Box::new(function) as Box<Runnable>;

        let process = runnable_to_process("/test", &br, false);

        let serialized_process = serde_json::to_string_pretty(&process).unwrap();
        assert_eq!(serialized_process, expected.replace("'", "\""));
    }
}