use std::fs;
use std::io::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use generator::cargo_gen;
use generator::main_gen;
use generator::runnables_gen;
use generator::functions_gen;
use model::flow::Flow;
use std::str;
use compiler::compile::CompilerTables;

// Return a string with the command and args required to compile and run the generated code
pub fn generate(flow: &Flow, output_dir: &PathBuf, log_level: &str, tables: &CompilerTables)
    -> Result<(String, Vec<String>)> {
    info!("Generating rust project into directory '{}'", output_dir.to_str().unwrap());

    let mut vars = vars_from_flow(flow);
    vars.insert("log_level".to_string(), log_level);

    let (cargo, args) = cargo_gen::create(&output_dir, &vars)?;
    let src_dir = create_src_dir(&output_dir)?;
    functions_gen::copy(&src_dir, &tables)?;
    main_gen::create(&src_dir, &mut vars, tables)?;
    runnables_gen::create(&src_dir, tables)?;

    Ok((cargo, args))
}

fn create_src_dir(root: &PathBuf) -> Result<PathBuf> {
    let mut dir = root.clone();
    dir.push("src");
    if !dir.exists() {
        fs::create_dir(&dir)?;
    }
    Ok(dir)
}

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