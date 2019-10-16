use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

use flowrlib::errors::Result;
use flowrlib::provider::Provider;
use url::Url;

/// The `FileProvider` implements the `Provider` trait and takes care of fetching content located
/// on the local file system.
pub struct FileProvider;

impl Provider for FileProvider {
    fn resolve_url(&self, url_str: &str, default_filename: &str, extensions: &[&str]) -> Result<(String, Option<String>)> {
        let url = Url::parse(url_str)
            .map_err(|_| format!("Could not convert '{}' to Url", url_str))?;
        let mut path = url.to_file_path()
            .map_err(|_| format!("Could not convert '{}' to a file path", url))?;
        let md_result = fs::metadata(&path)
            .map_err(|_| format!("Error getting file metadata for path: '{}'", path.display()));

        match md_result {
            Ok(md) => {
                if md.is_dir() {
                    debug!("'{}' is a directory, so attempting to find default file named '{}' in it",
                           path.display(), default_filename);
                    let file_found_url = FileProvider::find_file(&mut path, default_filename, extensions)?;
                    Ok((file_found_url, None))
                } else {
                    if md.is_file() {
                        return Ok((url.to_string(), None));
                    } else {
                        let file_found_url = FileProvider::file_by_extensions(&path, extensions)?;
                        return Ok((file_found_url, None));
                    }
                }
            }
            _ => { // doesn't exist
                let file_found_url = FileProvider::file_by_extensions(&path, extensions)?;
                return Ok((file_found_url, None));
            }
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
    /// Passed a path to a directory, it searches for a file in the directory called 'default_filename'
    /// If found, it opens the file and returns its contents as a String in the result
    pub fn find_file(dir: &PathBuf, default_filename: &str, extensions: &[&str]) -> Result<String> {
        let mut file = dir.clone();
        file.push(default_filename);

        Self::file_by_extensions(&file, extensions)
    }

    /// Given a path to a filename, try to find an existing file with any of the allowed extensions
    pub fn file_by_extensions(file: &PathBuf, extensions: &[&str]) -> Result<String> {
        let mut file_with_extension = file.clone();

        // for that file path, try with all the allowed file extensions
        for extension in extensions {
            file_with_extension.set_extension(extension);
            debug!("Looking for file '{}'", file_with_extension.display());
            if let Ok(md) = fs::metadata(&file_with_extension) {
                if md.is_file() {
                    let file_path_as_url = Url::from_file_path(&file_with_extension)
                        .map_err(|_| format!("Could not create url from file path '{}'",
                                             file_with_extension.to_str().unwrap()))?;

                    return Ok(file_path_as_url.to_string());
                }
            }
        }

        bail!("No file found at path '{}' with any of these extensions '{:?}'", file.display(), extensions)
    }
}

#[cfg(test)]
mod test {
    use std::ffi::OsStr;
    use std::path::Path;
    use std::path::PathBuf;

    use flowrlib::provider::Provider;

    use super::FileProvider;

    #[test]
    fn get_default_sample() {
        let mut path = PathBuf::from("../samples/hello-world").canonicalize().unwrap();
        match FileProvider::find_file(&mut path, "context", &["toml"]) {
            Ok(path_string) => {
                let path = Path::new(&path_string);
                assert_eq!(Some(OsStr::new("context.toml")), path.file_name());
            }
            _ => assert!(false, "Could not find_file 'context.toml'"),
        }
    }

    #[test]
    fn resolve_url_file_not_found() {
        let provider: &dyn Provider = &FileProvider;
        let url = "file://directory";
        let _ = provider.resolve_url(url, "default", &["toml"]);
    }

    #[test]
    #[should_panic]
    fn get_contents_file_not_found() {
        let provider: &dyn Provider = &FileProvider;
        provider.get_contents("file:///no-such-file").unwrap();
    }
}
