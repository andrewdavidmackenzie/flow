use std::io::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use model::flow::Flow;
use std::str;
use compiler::compile::CompilerTables;
use generator::rust_gen::generator;

/*
    All code generators should implement this method
*/
pub trait CodeGenerator {
    fn generate(output_dir: &PathBuf, vars: &mut HashMap<String, &str>, tables: &CompilerTables)
                -> Result<((String, Vec<String>), (String, Vec<String>))>;
}

/*
 Generate code using the specified generator and return two tuples of string + array of strings:
  1) command to build the project and array of args for the build command
  2) command to run the project and array of args for the run command
*/
pub fn generate(flow: &Flow, output_dir: &PathBuf, log_level: &str, tables: &CompilerTables,
                generator: &str) -> Result<((String, Vec<String>), (String, Vec<String>))> {
    info!("Generating project into directory '{}' using '{}' generator",
          output_dir.to_str().unwrap(), generator);

    let mut vars = vars_from_flow(flow);
    vars.insert("log_level".to_string(), log_level);

    generator::RustGenerator::generate(&output_dir, &mut vars, &tables )
}

/*
    Extract a set of known variables from the flow and create a table of variables with them
    for use in the code generation
*/
fn vars_from_flow(flow: &Flow) -> HashMap<String, &str> {
    let mut vars = HashMap::<String, &str>::new();
    let version = "0.0.0";
    let author_name = "Andrew Mackenzie";  // TODO make a variable
    let author_email = "andrew@mackenzie-serres.net"; // TODO make a variable

    vars.insert("package_name".to_string(), &flow.name);
    vars.insert("version".to_string(), version);

    if !author_name.is_empty() { // TODO FIX
        vars.insert("author_name".to_string(), author_name);
    }

    if !author_email.is_empty() {  // TODO FIX
        vars.insert("author_email".to_string(), author_email);
    }

    vars.insert("binary_name".to_string(), &flow.name);
    vars.insert("main_filename".to_string(), "main.rs");

    // TODO this just assumes flowstdlib is always used for now
    vars.insert("libraries".to_string(), "flowstdlib = \"~0.3\"");

    vars
}