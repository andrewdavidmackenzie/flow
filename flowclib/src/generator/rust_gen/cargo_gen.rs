use strfmt::Result as FmtResult;
use strfmt::strfmt;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::Result;
use std::path::PathBuf;

const CARGO_TEMPLATE: &'static str = "
[package]
name = \"{package_name}\"
version = \"{version}\"
authors = [\"{author_name} <{author_email}>\"]

[[bin]]
name = \"{binary_name}\"
path = \"src/{main_filename}\"

[dependencies]
flowrlib = {{path = \"../../../flowrlib\", version = \"~0.4.0\"}}
{libraries}
log = \"0.3.8\"
simplog = \"1.0.2\"

[workspace]
exclude = [\"..\"]
";

pub fn create(root: &PathBuf, vars: &HashMap<String, &str>)
    -> Result<((String, Vec<String>), (String, Vec<String>))> {
    let mut cargo_path = root.clone();
    cargo_path.push("Cargo.toml");
    let mut cargo_file = File::create(&cargo_path)?;
    cargo_file.write_all(contents(&vars).unwrap().as_bytes())?;
    // command and array of args to run this cargo file
    Ok((("cargo".to_string(),
        vec!("build".to_string(),
             "--manifest-path".to_string(),
             format!("{}/Cargo.toml", root.to_str().unwrap()))),
       ("cargo".to_string(),
        vec!("run".to_string(),
             "--manifest-path".to_string(),
             format!("{}/Cargo.toml", root.to_str().unwrap())))))
}

fn contents(vars: &HashMap<String, &str>) -> FmtResult<String> {
    strfmt(CARGO_TEMPLATE, &vars)
}

#[test]
fn cargo_gen_works() {
    let mut vars = HashMap::new();
    vars.insert("package_name".to_string(), "test-gen");
    vars.insert("version".to_string(), "0.0.0");
    vars.insert("author_name".to_string(), "Andrew Mackenzie");
    vars.insert("author_email".to_string(), "andrew@mackenzie-serres.net");
    vars.insert("binary_name".to_string(), "test-gen");
    vars.insert("main_filename".to_string(), "main.rs");
    vars.insert("libraries".to_string(), "");

    let output = contents(&vars).unwrap();
    assert!(output.contains("test-gen"));
    assert!(!output.contains("{package_name}"));
}
