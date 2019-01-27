use std::io::Result;
use std::collections::HashMap;
use std::collections::HashSet;
use model::runnable::Runnable;
use model::flow::Flow;
use model::route::Route;
use model::connection::Connection;
use flowrlib::manifest::Manifest;

#[derive(Serialize)]
pub struct GenerationTables {
    pub connections: Vec<Connection>,
    pub source_routes: HashMap<Route, (Route, usize)>,
    pub destination_routes: HashMap<Route, (usize, usize)>,
    pub collapsed_connections: Vec<Connection>,
    pub runnables: Vec<Box<Runnable>>,
    pub libs: HashSet<String>
}

serialize_trait_object!(Runnable);

/*
Things to add to the manifest
    - flow.alias
    - flow.version
    - flow.author_name
    - flow.author_email
*/

impl GenerationTables {
    pub fn new() -> Self {
        GenerationTables {
            connections: Vec::new(),
            source_routes: HashMap::<Route, (String, usize)>::new(),
            destination_routes: HashMap::<Route, (usize, usize)>::new(),
            collapsed_connections: Vec::new(),
            runnables: Vec::new(),
            libs: HashSet::new()
        }
    }
}

pub fn create_manifest(_flow: &Flow, out_dir_path: &str, tables: &GenerationTables) -> Result<Manifest> {
    let mut manifest = Manifest::new();
    let mut base_path = out_dir_path.to_string();
    base_path.push('/');

    // Generate runtime Process struct for each of the runnables
    for runnable in &tables.runnables {
        manifest.processes.push(runnable_to_process(&base_path, runnable));
    }

    Ok(manifest)
}

// Do as an Into trait?
fn runnable_to_process(out_dir_path: &str, runnable: &Box<Runnable>) -> flowrlib::process::Process {
    let name = runnable.alias();
    let is_static = runnable.is_static_value();
    let mut impl_path = runnable.get_impl_path();

    // make path to implementation relative to the output directory if under it
    impl_path = impl_path.replace(out_dir_path, "");

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

    flowrlib::process::Process::new(
        name,
        is_static,
        impl_path,
        input_depths,
        id,
        initial_value,
        output_routes,
    )
}

// TODO re-instate tests with new implementation

/*
#[cfg(test)]
mod test {
    use serde_json::Value as JsonValue;
    use model::value::Value;
    use model::io::IO;
    use model::function::Function;
    use model::runnable::Runnable;
    use url::Url;

    #[test]
    #[ignore]
    fn test_value_to_code() {
        let value = Value::new("value".to_string(),
                               "String".to_string(),
                               Some(JsonValue::String("Hello-World".to_string())),
                               false,
                               "/flow0/value".to_string(),
                               Some(vec!(IO::new(&"Json".to_string(), &"".to_string()))),
                               vec!(("".to_string(), 1, 0)),
                               1);

        let br = Box::new(value) as Box<Runnable>;
        let code = runnable_to_code(&br);
        assert_eq!(code, "Process::new(\"value\", 1, false, vec!(1, ), 1, &Fifo{}, Some(json!(\"Hello-World\")), vec!((\"\".to_string(), 1, 0),))")
    }

    #[test]
    #[ignore]
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

        let br = Box::new(value) as Box<Runnable>;
        let code = runnable_to_code(&br);
        assert_eq!(code, "Process::new(\"value\", 1, true, vec!(1, ), 1, &Fifo{}, Some(json!(\"Hello-World\")), vec!((\"\".to_string(), 1, 0),))")
    }

    #[test]
    #[ignore]
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

        let br = Box::new(value) as Box<Runnable>;
        let code = runnable_to_code(&br);
        assert_eq!(code, "Process::new(\"value\", 1, false, vec!(1, ), 1, &Fifo{}, Some(json!(\"Hello-World\")), vec!((\"\".to_string(), 1, 0),(\"/sub_route\".to_string(), 2, 0),))")
    }

    #[test]
    #[ignore]
    fn function_with_sub_route_output_to_code() {
        let function = Function::new(
            "Stdout".to_string(),
            "print".to_string(),
            Some(vec!()),
            Some(vec!(
                IO::new(&"Json".to_string(), &"".to_string()),
                IO::new(&"String".to_string(), &"".to_string())
            )),
            Url::parse("file:///fake/file").unwrap(),
            "/flow0/stdout".to_string(),
            None,
            vec!(("".to_string(), 1, 0), ("sub_route".to_string(), 2, 0)),
            0);

        let br = Box::new(function) as Box<Runnable>;
        let code = runnable_to_code(&br);
        assert_eq!(code, "Process::new(\"print\", 0, false, vec!(), 0, &Stdout{}, None, vec!((\"\".to_string(), 1, 0),(\"/sub_route\".to_string(), 2, 0),))")
    }

    #[test]
    #[ignore]
    fn function_to_code() {
        let function = Function::new(
            "Stdout".to_string(),
            "print".to_string(),
            Some(vec!()),
            Some(vec!(
                IO::new(&"String".to_string(), &"".to_string())
            )),
            Url::parse("file:///fake/file").unwrap(),
            "/flow0/stdout".to_string(),
            None,
            vec!(("".to_string(), 1, 0)),
            0);

        let br = Box::new(function) as Box<Runnable>;
        let code = runnable_to_code(&br);
        assert_eq!(code, "Process::new(\"print\", 0, false, vec!(), 0, &Stdout{}, None, vec!((\"\".to_string(), 1, 0),))")
    }

    #[test]
    #[ignore]
    fn function_with_array_element_output() {
        let function = Function::new(
            "Stdout".to_string(),
            "print".to_string(),
            Some(vec!()),
            Some(vec!(
                IO::new(&"Array".to_string(), &"".to_string())
            )),
            Url::parse("file:///fake/file").unwrap(),
            "/flow0/stdout".to_string(),
            None,
            vec!(("0".to_string(), 1, 0)),
            0);

        let br = Box::new(function) as Box<Runnable>;
        let code = runnable_to_code(&br);
        assert_eq!(code, "Process::new(\"print\", 0, false, vec!(), 0, &Stdout{}, None, vec!((\"/0\".to_string(), 1, 0),))")
    }
}
*/