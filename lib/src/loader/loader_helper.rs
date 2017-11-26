use std::path::PathBuf;
use loader::yaml_loader::FlowYamlLoader;
use loader::toml_loader::FlowTomelLoader;
use loader::loader::Loader;

const TOML: &Loader = &FlowTomelLoader {} as &Loader;
const YAML: &Loader = &FlowYamlLoader {} as &Loader;

pub fn get_loader(file_path: &PathBuf) -> Result<&'static Loader, String> {
    match file_path.extension() {
        Some(ext) => {
            match ext.to_str() {
                Some("toml") => Ok(TOML),
                Some("yaml") => Ok(YAML),
                _ => Err("Unknown file extension so cannot determine loader to use".to_string())
            }
        }
        None => Err("No file extension so cannot determine loader to use".to_string())
    }
}

#[test]
#[should_panic]
fn no_extension() {
    get_loader(&PathBuf::from("no_extension")).unwrap();
}

#[test]
#[should_panic]
fn invalid_extension() {
    get_loader(&PathBuf::from("no_extension.wrong")).unwrap();
}

#[test]
fn valid_extension() {
    get_loader(&PathBuf::from("OK.toml")).unwrap();
}
