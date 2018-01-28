use loader::yaml_loader::FlowYamlLoader;
use loader::toml_loader::FlowTomelLoader;
use loader::loader::Loader;
use url::Url;

const TOML: &Loader = &FlowTomelLoader as &Loader;
const YAML: &Loader = &FlowYamlLoader as &Loader;

pub fn get_loader(url: &Url) -> Result<&'static Loader, String> {
    match get_file_extension(url) {
        Ok(ext) => {
            match ext.as_ref() {
                "toml" => Ok(TOML),
                "yaml" => Ok(YAML),
                "yml" => Ok(YAML),
                _ => Err("Unknown file extension so cannot determine which loader to use".to_string())
            }
        }
        Err(e) => Err(format!("Cannot determine which loader to use ({})", e.to_string())
        )
    }
}

fn get_file_extension(url: &Url) -> Result<String, String> {
    let segments = url.path_segments().ok_or_else(|| "cannot be base")?;
    let last_segment = segments.last().ok_or_else(|| "no segments")?;
    let splits: Vec<&str> = last_segment.split('.').collect();
    if splits.len() < 2 {
        Err("No file extension".to_string())
    } else {
        Ok(splits.last().unwrap().to_string())
    }
}

#[cfg(test)]
mod test {
    use url::Url;
    use super::get_file_extension;
    use super::get_loader;

    #[test]
    #[should_panic]
    fn no_extension() {
        get_file_extension(&Url::parse("file:///no_extension").unwrap()).unwrap();
    }

    #[test]
    fn valid_file_extension() {
        get_file_extension(&Url::parse("file::///OK.toml").unwrap()).unwrap();
    }

    #[test]
    fn valid_http_extension() {
        get_file_extension(&Url::parse("http://test.com/OK.toml").unwrap()).unwrap();
    }

    #[test]
    #[should_panic]
    fn invalid_extension() {
        get_loader(&Url::parse("file:///extension.wrong").unwrap()).unwrap();
    }

    #[test]
    fn toml_extension_loader() {
        get_loader(&Url::parse("file:///extension.toml").unwrap()).unwrap();
    }

    #[test]
    fn yaml_extension_loader() {
        get_loader(&Url::parse("file:///extension.yaml").unwrap()).unwrap();
    }
}