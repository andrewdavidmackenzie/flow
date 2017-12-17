use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::Result;
use std::collections::HashMap;
use compiler::generator::cargo_gen::cargo_file_contents;
// TODO
use compiler::generator::main_gen::main_file_contents;
// TODO
use compiler::generator::runnables_gen::runnables_file_contents;
// TODO
use flowrlib::runnable::Runnable;
use model::flow::Flow;

pub fn generate(flow: &mut Flow, _overwrite: bool, log_level: &str,
                runnables: Vec<Box<Runnable>>) -> Result<()> {
    let mut directory = flow.source.clone();
    directory.pop();
    info!("Generating rust project into dir '{}'", directory.to_str().unwrap());

    // create the directory - if doesn't already exist
    if !directory.exists() {
        fs::create_dir(&directory)?;
    }

    let mut vars = vars_from_flow(flow);

    // write the cargo file into the root
    directory.push("Cargo.toml");
    let mut cargo = File::create(&directory)?;
    cargo.write_all(cargo_file_contents(&vars).unwrap().as_bytes())?;
    directory.pop();

    // create the src subdir
    directory.push("src");
    if !directory.exists() {
        fs::create_dir(&directory)?;
    }

    // write the main.rs file into src
    directory.push("main.rs");
    let mut main_rs = File::create(&directory)?;
    vars.insert("log_level".to_string(), log_level);
    main_rs.write_all(main_file_contents(&vars).unwrap().as_bytes())?;
    directory.pop();

    // write the runnable.rs file into src
    directory.push("runnables.rs");
    let mut runnables_rs = File::create(&directory)?;
    runnables_rs.write_all(runnables_file_contents(&vars, runnables).unwrap().as_bytes())?;

    Ok(())
}

fn vars_from_flow(flow: &mut Flow) -> HashMap<String, &str> {
    let mut vars = HashMap::<String, &str>::new();
    let version = "0.0.0";
    let author_name = "Andrew Mackenzie";
    let author_email = "andrew@mackenzie-serres.net";

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

    vars
}