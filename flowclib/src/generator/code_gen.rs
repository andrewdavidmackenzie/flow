use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use generator::cargo_gen;
use generator::main_gen;
use generator::runnables_gen;
use flowrlib::runnable::Runnable;
use model::flow::Flow;
use std::str;
use std::collections::HashSet;

pub fn generate(flow: &Flow, output_dir: PathBuf, log_level: &str,
                libs: HashSet<String>, lib_references: HashSet<String>,
                runnables: Vec<Box<Runnable>>) -> Result<String> {
    info!("Generating rust project into directory '{}'", output_dir.to_str().unwrap());

    let mut dir = output_dir.clone();
    let mut vars = vars_from_flow(flow);

    let crates = crates(libs);
    let lib_refs = lib_refs(lib_references);

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
    main_rs.write_all(main_gen::contents(&vars, crates).unwrap().as_bytes())?;
    dir.pop();

    // write the runnable.rs file into src
    dir.push("runnables.rs");
    let mut runnables_rs = File::create(&dir)?;
    runnables_rs.write_all(runnables_gen::contents(&vars,
                                                   lib_refs,
                                                   runnables).unwrap().as_bytes())?;

    Ok(format!("run command 'cargo run --manifest-path {}/Cargo.toml' to compile and run generated project",
               output_dir.to_str().unwrap()))
}

fn crates(libs: HashSet<String>) -> Vec<String>{
    let mut crates: Vec<String> = Vec::new();
    for lib in libs {
        crates.push(format!("extern crate {};\n", lib));
    }

    crates
}

fn lib_refs(libs_references: HashSet<String>) -> Vec<String>{
    let mut lib_refs: Vec<String> = Vec::new();
    for lib_ref in libs_references {
        let lib_use = str::replace(&lib_ref, "/", "::");
        lib_refs.push(format!("use {};\n", lib_use));
    }

    lib_refs
}

fn vars_from_flow(flow: &Flow) -> HashMap<String, &str> {
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