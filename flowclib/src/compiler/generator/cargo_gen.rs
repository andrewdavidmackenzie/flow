use strfmt::Result;
use strfmt::strfmt;
use std::collections::HashMap;

const CARGO_TEMPLATE: &'static str = "
[package]
name = \"{package_name}\"
version = \"{version}\"
authors = [\"{author_name} <{author_email}>\"]

[[bin]]
name = \"{binary_name}\"
path = \"src/{main_filename}\"

[dependencies]
flowrlib = {{ path = \"/Users/andrew/workspace/flow/flowrlib\", version = \"*\" }}
flowstdlib = {{ path = \"/Users/andrew/workspace/flow/flowstdlib\", version = \"*\"}}
log = \"*\"

[workspace]
exclude = [\"..\"]
";

pub fn cargo_file_contents(vars: &HashMap<String, &str>) -> Result<String> {
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

    let output = cargo_file_contents(&vars).unwrap();
    assert!(output.contains("test-gen"));
    assert!(!output.contains("{package_name}"));
}
