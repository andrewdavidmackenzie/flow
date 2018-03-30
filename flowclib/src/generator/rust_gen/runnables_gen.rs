use std::io::Result;
use strfmt::strfmt;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::collections::HashSet;
use std::io::{Error, ErrorKind};
use generator::code_gen::CodeGenTables;
use model::runnable::Runnable;

const RUNNABLES_PREFIX: &'static str = "
// Flow Run-time library references
use flowrlib::runnable::Runnable;
{value_used}
{value_implementation_used}
{function_used}

// Rust std library references
use std::sync::{{Arc, Mutex}};\n";

const GET_RUNNABLES: &'static str = "
pub fn get_runnables() -> Vec<Arc<Mutex<Runnable>>> {{
    let mut runnables = Vec::<Arc<Mutex<Runnable>>>::with_capacity({num_runnables});\n\n";

const RUNNABLES_SUFFIX: &'static str = "
    runnables
}";

// Create the 'runnables.rs' file in the output project's source folder
pub fn create(src_dir: &PathBuf, tables: &CodeGenTables)
              -> Result<()> {
    let lib_refs = lib_refs(&tables.lib_references);
    let mut file = src_dir.clone();
    file.push("runnables.rs");
    let mut runnables_rs = File::create(&file)?;
    let contents = contents(tables, &lib_refs);
    runnables_rs.write_all(contents.unwrap().as_bytes())
}

fn uses(runnable_type: &str, runnables: &Vec<Box<Runnable>>) -> bool {
    for runnable in runnables {
        if runnable.get_type() == runnable_type {
            return true;
        }
    }
    return false;
}

fn contents(tables: &CodeGenTables, lib_refs: &Vec<String>) -> Result<String> {
    let num_runnables = &tables.runnables.len().to_string();
    let mut vars = HashMap::new();

    if uses("Value", &tables.runnables) {
        vars.insert("value_used".to_string(), "use flowrlib::value::Value;");
        vars.insert("value_implementation_used".to_string(), "use flowstdlib::zero_fifo::Fifo;");
    } else {
        vars.insert("value_used".to_string(), "");
        vars.insert("value_implementation_used".to_string(), "");
    }

    if uses("Function", &tables.runnables) {
        vars.insert("function_used".to_string(), "use flowrlib::function::Function;");
    } else {
        vars.insert("function_used".to_string(), "");
    }

    let mut content = strfmt(RUNNABLES_PREFIX, &vars).unwrap();

    content.push_str("\n// Library functions\n");
    for lib_ref in lib_refs {
        content.push_str(&lib_ref);
    }

    // Add "use" statements for functions referenced
    content.push_str(&usages(&tables.runnables).unwrap());

    // add declaration of runnables array etc - parameterized by the number of runnables
    vars = HashMap::<String, &str>::new();
    vars.insert("num_runnables".to_string(), num_runnables);
    content.push_str(&strfmt(GET_RUNNABLES, &vars).unwrap());

    // Generate code for each of the runnables
    for runnable in &tables.runnables {
        let run_str = format!("    runnables.push(Arc::new(Mutex::new({})));\n",
                              runnable_to_code(runnable));
        content.push_str(&run_str);
    }

    // return the array of runnables - No templating in this part (at the moment)
    content.push_str(RUNNABLES_SUFFIX);

    Ok(content)
}

// add use clauses for functions that are part of the flow, not library functions
// "use module::Functionname;" e.g. "use reverse::Reverse;"
fn usages(runnables: &Vec<Box<Runnable>>) -> Result<String> {
    let mut usages_string = String::new();

    // Find all the functions that are not loaded from libraries
    for runnable in runnables {
        if let Some(source_url) = runnable.source_url() {
            let source = source_url.to_file_path()
                .map_err(|_e| Error::new(ErrorKind::InvalidData, "Could not convert to file path"))?;
            let usage = source.file_stem().unwrap();
            usages_string.push_str(&format!("use {}::{};\n",
                                            usage.to_str().unwrap(), runnable.name()));
        }
    }

    Ok(usages_string)
}

// Convert a set of libraries used in all the flows in '/' format into use statements of rust
fn lib_refs(libs_references: &HashSet<String>) -> Vec<String> {
    let mut lib_refs: Vec<String> = Vec::new();
    for lib_ref in libs_references {
        let lib_use = str::replace(&lib_ref, "/", "::");
        lib_refs.push(format!("use {};\n", lib_use));
    }

    lib_refs
}

// Output a statement that instantiates an instance of the Runnable type used, that can be used
// to build the list of runnables
fn runnable_to_code(runnable: &Box<Runnable>) -> String {
    let mut code = format!("{}::new(\"{}\".to_string(), ", runnable.get_type(), runnable.name());
    match &runnable.get_inputs() {
        &None => code.push_str(&format!("{}, ", 0)),
        &Some(ref inputs) => code.push_str(&format!("{}, ", inputs.len()))
    }
    code.push_str(&format!("{}, Box::new({}{{}}), ", runnable.get_id(), runnable.get_implementation()));

    code.push_str(&format!("{},",  match runnable.get_initial_value() {
        None => "None".to_string(),
        Some(value) => format!("Some(json!({}))", value.to_string())
    }));

    // Add tuples of this function's output routes to runnables and the input it's connected to
    code.push_str(" vec!(");
    debug!("Runnable '{}' output routes: {:?}", runnable.name(), runnable.get_output_routes());
    for ref route in runnable.get_output_routes() {
        if route.0.is_empty() {
            code.push_str(&format!("(\"\", {}, {}),", route.1, route.2));
        } else {
            code.push_str(&format!("(\"/{}\", {}, {}),", route.0, route.1, route.2));
        }
    }
    code.push_str(")");

    code.push_str(")");

    code
}

#[cfg(test)]
mod test {
    use serde_json::Value as JsonValue;
    use model::value::Value;
    use model::io::IO;
    use model::function::Function;
    use model::runnable::Runnable;
    use url::Url;
    use super::runnable_to_code;

    #[test]
    fn test_value_to_code() {
        let value = Value {
            name: "value".to_string(),
            datatype: "String".to_string(),
            value: Some(JsonValue::String("Hello-World".to_string())),
            route: "/flow0/value".to_string(),
            outputs: Some(vec!(IO {
                name: "".to_string(),
                datatype: "Json".to_string(),
                route: "".to_string(),
                flow_io: false })),
            output_connections: vec!(("".to_string(), 1, 0)),
            id: 1,
        };

        let br = Box::new(value) as Box<Runnable>;
        let code = runnable_to_code(&br);
        assert_eq!(code, "Value::new(\"value\".to_string(), 1, 1, Box::new(Fifo{}), Some(json!(\"Hello-World\")), vec!((\"\", 1, 0),))")
    }

    #[test]
    fn value_with_sub_route_output_to_code() {
        let value = Value {
            name: "value".to_string(),
            datatype: "String".to_string(),
            value: Some(JsonValue::String("Hello-World".to_string())),
            route: "/flow0/value".to_string(),
            outputs: Some(vec!(
                IO { name: "".to_string(), datatype: "Json".to_string(), route: "".to_string(), flow_io: false },
                IO { name: "sub_route".to_string(), datatype: "String".to_string(), route: "".to_string(), flow_io: false }
            )),
            output_connections: vec!(("".to_string(), 1, 0), ("sub_route".to_string(), 2, 0)),
            id: 1,
        };

        let br = Box::new(value) as Box<Runnable>;
        let code = runnable_to_code(&br);
        assert_eq!(code, "Value::new(\"value\".to_string(), 1, 1, Box::new(Fifo{}), Some(json!(\"Hello-World\")), vec!((\"\", 1, 0),(\"/sub_route\", 2, 0),))")
    }

    #[test]
    fn function_with_sub_route_output_to_code() {
        let function = Function {
            name: "Stdout".to_string(),
            inputs: Some(vec!()),
            outputs: Some(vec!(
                IO { name: "".to_string(), datatype: "Json".to_string(), route: "".to_string(), flow_io: false },
                IO { name: "sub_route".to_string(), datatype: "String".to_string(), route: "".to_string(), flow_io: false }
            )),
            source_url: Url::parse("file:///fake/file").unwrap(),
            route: "/flow0/stdout".to_string(),
            lib_reference: None,
            output_connections: vec!(("".to_string(), 1, 0), ("sub_route".to_string(), 2, 0)),
            id: 0,
        };

        let br = Box::new(function) as Box<Runnable>;
        let code = runnable_to_code(&br);
        assert_eq!(code, "Function::new(\"Stdout\".to_string(), 0, 0, Box::new(Stdout{}), None, vec!((\"\", 1, 0),(\"/sub_route\", 2, 0),))")
    }
}