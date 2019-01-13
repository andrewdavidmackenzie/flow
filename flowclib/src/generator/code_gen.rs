use std::io::Result;
use std::io::{Error, ErrorKind};
use std::collections::HashMap;
use std::path::PathBuf;
use model::flow::Flow;
use std::str;
use generator::rust_gen::generator::RustGenerator;
use model::connection::Connection;
use std::collections::HashSet;
use model::runnable::Runnable;
use model::route::Route;

const RUST: &CodeGenerator = &RustGenerator as &CodeGenerator;

#[derive(Serialize)]
pub struct CodeGenTables {
    pub connections: Vec<Connection>,
    pub source_routes: HashMap<Route, (Route, usize)>,
    pub destination_routes: HashMap<Route, (usize, usize)>,
    pub collapsed_connections: Vec<Connection>,
    pub runnables: Vec<Box<Runnable>>,
    pub libs: HashSet<String>,
    pub lib_references: HashSet<String>,
}

serialize_trait_object!(Runnable);

impl CodeGenTables {
    pub fn new() -> Self {
        CodeGenTables {
            connections: Vec::new(),
            source_routes: HashMap::<Route, (String, usize)>::new(),
            destination_routes: HashMap::<Route, (usize, usize)>::new(),
            collapsed_connections: Vec::new(),
            runnables: Vec::new(),
            libs: HashSet::new(),
            lib_references: HashSet::new(),
        }
    }
}

/*
    All code generators should implement this method
*/
pub trait CodeGenerator {
    fn generate(&self, output_dir: &PathBuf, vars: &mut HashMap<String, &str>, tables: &CodeGenTables)
                -> Result<((String, Vec<String>), (String, Vec<String>))>;
}

/*
 Generate code using the specified generator and return two tuples of string + array of strings:
  1) command to build the project and array of args for the build command
  2) command to run the project and array of args for the run command
*/
pub fn generate(flow: &Flow, output_dir: &PathBuf, log_level: &str, tables: &CodeGenTables,
                extension: &str) -> Result<((String, Vec<String>), (String, Vec<String>))> {
    let mut vars = vars_from_flow(flow);
    vars.insert("log_level".to_string(), log_level);

    let generator = get_generator(extension)?;
    info!("Generating project into directory '{}' using '{}' generator",
          output_dir.to_str().unwrap(), extension);
    generator.generate(&output_dir, &mut vars, &tables)
}

fn get_generator(extension: &str) -> Result<&'static CodeGenerator> {
    match extension {
        "rs" => Ok(RUST),
        _ => Err(Error::new(ErrorKind::InvalidData,
                            format!("Could not find a code generator for extension '{}'", extension)))
    }
}

/*
    Extract a set of known variables from the flow and create a table of variables with them
    for use in the code generation
*/
fn vars_from_flow(flow: &Flow) -> HashMap<String, &str> {
    let mut vars = HashMap::<String, &str>::new();

    vars.insert("package_name".to_string(), &flow.alias);
    vars.insert("package_version".to_string(), &flow.version);
    vars.insert("author_name".to_string(), &flow.author_name);
    vars.insert("author_email".to_string(), &flow.author_email);
    vars.insert("binary_name".to_string(), &flow.alias);
    vars.insert("main_filename".to_string(), "main.rs");

    // TODO this just assumes flowstdlib is always used for now
    vars.insert("libraries".to_string(),
                "flowstdlib = {path = \"../../../flowstdlib\", version = \"~0.7.0\"}");
    vars
}

#[cfg(test)]
mod test {
    use super::get_generator;

    #[test]
    fn code_generator_for_rust() {
        get_generator("rs").unwrap();
    }

    #[test]
    #[should_panic]
    fn no_code_generator_for_fake() {
        get_generator("fake").unwrap();
    }
}