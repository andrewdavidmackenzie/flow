use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

use url::Url;

use flowrlib::errors::Result;
use flowrlib::provider::Provider;

pub struct FileProvider;

impl Provider for FileProvider {
    fn resolve_url(&self, url_str: &str, default_filename: &str, extensions: &[&str]) -> Result<(String, Option<String>)> {
        let url = Url::parse(url_str)
            .map_err(|_| format!("Could not convert '{}' to Url", url_str))?;
        let mut path = url.to_file_path().unwrap();
        let md = fs::metadata(&path)
            .map_err(|_| format!("Error getting file metadata for path: '{}'", path.display()))?;

        if md.is_dir() {
            info!("'{}' is a directory, so attempting to find default file named '{}' in it",
                  path.display(), default_filename);
            let resolved_url = FileProvider::find_file(&mut path, default_filename, extensions)?;
            Ok((resolved_url, None))
        } else {
            Ok((url.to_string(), None))
        }
    }

    fn get_contents(&self, url_str: &str) -> Result<Vec<u8>> {
        let url = Url::parse(url_str)
            .map_err(|_| format!("Could not convert '{}' to Url", url_str))?;
        let file_path = url.to_file_path().unwrap();
        let mut f = File::open(&file_path)
            .map_err(|_| format!("Could not open file '{:?}'", file_path))?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)
            .map_err(|_| format!("Could not read content from '{:?}'", file_path))?;
        Ok(buffer)
    }
}

impl FileProvider {
    /*
        Passed a path to a directory, it searches for a file in the directory called 'default_filename'
        If found, it opens the file and returns its contents as a String in the result
    */
    pub fn find_file(path: &PathBuf, default_filename: &str, extensions: &[&str]) -> Result<String> {
        let mut file = path.clone();
        file.push(default_filename);

        for extension in extensions {
            file.set_extension(extension);
            let md = fs::metadata(&file)
                .map_err(|_| format!("Could not get metadata for file '{}'", file.display()))?;
            debug!("Looking for file '{}'", file.display());
            if md.is_file() {
                let file_path_as_url = Url::from_file_path(&file)
                    .map_err(|_| format!("Could not create url from file path '{}'",
                                         file.to_str().unwrap()))?;

                return Ok(file_path_as_url.to_string());
            }
        }

        bail!("No default file found with name '{}' Tried extensions '{:?}'",
               default_filename,
               extensions)
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use flowrlib::provider::Provider;

    use super::FileProvider;

    #[test]
    fn get_default_sample() {
        let mut path = PathBuf::from("../samples/hello-world");
        match FileProvider::find_file(&mut path, "context", &["toml"]) {
            Ok(path) => {
                assert_eq!("context.toml", path);
            }
            _ => assert!(false),
        }
    }

    #[test]
    #[should_panic]
    fn get_contents_file_not_found() {
        let provider: &dyn Provider = &FileProvider;
        provider.get_contents("file:///no-such-file").unwrap();
    }
}
