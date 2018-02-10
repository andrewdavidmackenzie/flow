use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use compiler::generator::cargo_gen;
use compiler::generator::main_gen;
use compiler::generator::runnables_gen;
use flowrlib::runnable::Runnable;
use model::flow::Flow;

pub fn generate(flow: &mut Flow, output_dir: &PathBuf, log_level: &str,
                runnables: Vec<Box<Runnable>>) -> Result<()> {
    let mut dir = output_dir.clone();
    let mut vars = vars_from_flow(flow);

    // TODO - extract these from the flow definition.
    let mut library_references = Vec::new();
    let mut external_crates = Vec::new();
    external_crates.push("extern crate flowstdlib;\n");
    library_references.push("use flowstdlib::stdio::stdout::Stdout;\n");
    library_references.push("use flowstdlib::math::add::Add;\n");

    // write the cargo file into the root
    dir.push("Cargo.toml");
    let mut cargo = File::create(&dir)?;
    cargo.write_all(cargo_gen::contents(&vars).unwrap().as_bytes())?;
    dir.pop();

    // create the src subdir
    dir.push("src");
    if !dir.exists() {
        fs::create_dir(&dir)?;
    }

    // write the main.rs file into src
    dir.push("main.rs");
    let mut main_rs = File::create(&dir)?;
    vars.insert("log_level".to_string(), log_level);
    main_rs.write_all(main_gen::contents(&vars, external_crates).unwrap().as_bytes())?;
    dir.pop();

    // write the runnable.rs file into src
    dir.push("runnables.rs");
    let mut runnables_rs = File::create(&dir)?;
    runnables_rs.write_all(runnables_gen::contents(&vars,
                                                   library_references,
                                                   runnables).unwrap().as_bytes())?;

    Ok(())
}

fn vars_from_flow(flow: &mut Flow) -> HashMap<String, &str> {
    let mut vars = HashMap::<String, &str>::new();
    let version = "0.0.0";
    let author_name = "Andrew Mackenzie";  // TODO make a variable
    let author_email = "andrew@mackenzie-serres.net"; // TODO

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
    vars.insert("libraries".to_string(), "flowstdlib = { path = \"../../../flowstdlib\", version = \"~0.3\"} ");

    vars
}