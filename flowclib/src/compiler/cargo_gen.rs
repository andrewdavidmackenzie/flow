use strfmt::Result;
use strfmt::strfmt;
use std::collections::HashMap;

const CARGO_TEMPLATE: &'static str = "
[package]
name = '{package_name}'
version = '{version}'
authors = ['{author_name} <{author_email}>']

[[bin]]
name = '{binary_name}'
path = 'src/{main_filename}'

[dependencies]
flowrlib = { path = '../flowrlib', version = '*' }
flowstdlib = { path = '../flowstdlib', version = '*'}
log = '*'
";

pub fn cargo_file_contents(package_name: &str, version: &str, author_name: &str,
                           author_email: &str, binary_name: &str, main_filename: &str) -> Result<String>{
    let mut vars = HashMap::new();
    vars.insert("package_name".to_string(), package_name);
    vars.insert("version".to_string(), version);
    vars.insert("author_name".to_string(), author_name);
    vars.insert("author_email".to_string(), author_email);
    vars.insert("binary_name".to_string(), binary_name);
    vars.insert("main_filename".to_string(), main_filename);

    strfmt(CARGO_TEMPLATE, &vars)

//    format!(CARGO_TEMPLATE, package_name, version, author_name,
//            author_email, binary_name, main_filename)
}
