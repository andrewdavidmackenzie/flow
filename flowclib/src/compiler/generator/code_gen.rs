use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::Result;
use std::path::PathBuf;
use std::collections::HashMap;
use compiler::generator::cargo_gen::cargo_file_contents; // TODO
use compiler::generator::main_gen::main_file_contents; // TODO
use compiler::generator::runnables_gen::runnables_file_contents; // TODO

pub fn generate(mut directory: PathBuf, _overwrite: bool) -> Result<()> {
    let mut cargo_vars = HashMap::new();
    let package_name = "hello-gen";
    let binary_name = "hello-gen";
    let version = "0.0.0";
    let author_name = "Andrew Mackenzie";
    let author_email = "andrew@mackenzie-serres.net";

    cargo_vars.insert("package_name".to_string(), package_name);
    cargo_vars.insert("version".to_string(), version);

    if !author_name.is_empty() { // TODO FIX
        cargo_vars.insert("author_name".to_string(), author_name);
    }

    if !author_email.is_empty() {  // TODO FIX
        cargo_vars.insert("author_email".to_string(), author_email);
    }

    cargo_vars.insert("binary_name".to_string(), binary_name);

    cargo_vars.insert("main_filename".to_string(), "main.rs");

    // create the directory - if doesn't already exist
    if !directory.exists() {
        fs::create_dir(&directory)?;
    }

    // write the cargo file into the root
    directory.push("Cargo.toml");
    let mut cargo = File::create(&directory)?;
    cargo.write_all(cargo_file_contents(&cargo_vars).unwrap().as_bytes())?;
    directory.pop();

    // create the src subdir
    directory.push("src");
    if !directory.exists() {
        fs::create_dir(&directory)?;
    }

    // write the main.rs file into src
    directory.push("main.rs");
    let mut main = File::create(&directory)?;
    let mut main_vars = HashMap::new();
    main.write_all(main_file_contents(&main_vars).unwrap().as_bytes())?;
    directory.pop();

    // write the runnable.rs file into src
    directory.push("runnables.rs");
    let mut runnables = File::create(&directory)?;
    let mut runnables_vars = HashMap::new();
    runnables.write_all(runnables_file_contents(&runnables_vars).unwrap().as_bytes())?;

    Ok(())
}