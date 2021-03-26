use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use log::{debug, trace};
use url::Url;

use crate::errors::Result;

use super::provider::Provider;

/// The `FileProvider` implements the `Provider` trait and takes care of fetching content located
/// on the local file system.
pub struct FileProvider;

impl Provider for FileProvider {
    fn resolve_url(
        &self,
        url_str: &str,
        default_filename: &str,
        extensions: &[&str],
    ) -> Result<(String, Option<String>)> {
        let url =
            Url::parse(url_str).map_err(|_| format!("Could not convert '{}' to Url", url_str))?;
        let path = url
            .to_file_path()
            .map_err(|_| format!("Could not convert '{}' to a file path", url))?;
        let md_result = fs::metadata(&path)
            .map_err(|_| format!("Error getting file metadata for path: '{}'", path.display()));

        match md_result {
            Ok(md) => {
                if md.is_dir() {
                    trace!(
                        "'{}' is a directory, so attempting to find file with same name in it",
                        path.display()
                    );
                    if let Some(dir_os_name) = path.file_name() {
                        let dir_name = dir_os_name.to_string_lossy();
                        if let Ok(file_found_url) =
                            FileProvider::find_file(&path, &dir_name, extensions)
                        {
                            return Ok((file_found_url, None));
                        }
                    }

                    trace!(
                        "'{}' is a directory, so attempting to find default file named '{}' in it",
                        path.display(),
                        default_filename
                    );
                    let file_found_url =
                        FileProvider::find_file(&path, default_filename, extensions)?;
                    Ok((file_found_url, None))
                } else if md.is_file() {
                    Ok((url.to_string(), None))
                } else {
                    let file_found_url = FileProvider::file_by_extensions(&path, extensions)?;
                    Ok((file_found_url, None))
                }
            }
            _ => {
                // doesn't exist
                let file_found_url = FileProvider::file_by_extensions(&path, extensions)?;
                Ok((file_found_url, None))
            }
        }
    }

    fn get_contents(&self, url_str: &str) -> Result<Vec<u8>> {
        let url =
            Url::parse(url_str).map_err(|_| format!("Could not convert '{}' to Url", url_str))?;
        let file_path = url
            .to_file_path()
            .map_err(|_| "Could not convert Url to file path")?;
        let mut f =
            File::open(&file_path).map_err(|_| format!("Could not open file '{:?}'", file_path))?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)
            .map_err(|_| format!("Could not read content from '{:?}'", file_path))?;
        Ok(buffer)
    }
}

impl FileProvider {
    /// Passed a path to a directory, it searches for a file in the directory called 'default_filename'
    /// If found, it opens the file and returns its contents as a String in the result
    pub fn find_file(dir: &Path, default_filename: &str, extensions: &[&str]) -> Result<String> {
        let mut file = dir.to_path_buf();
        file.push(default_filename);

        Self::file_by_extensions(&file, extensions)
    }

    /// Given a path to a filename, try to find an existing file with any of the allowed extensions
    pub fn file_by_extensions(file: &Path, extensions: &[&str]) -> Result<String> {
        let mut file_with_extension = file.to_path_buf();

        // for that file path, try with all the allowed file extensions
        for extension in extensions {
            file_with_extension.set_extension(extension);
            debug!("Looking for file '{}'", file_with_extension.display());
            if let Ok(md) = fs::metadata(&file_with_extension) {
                if md.is_file() {
                    let file_path_as_url =
                        Url::from_file_path(&file_with_extension).map_err(|_| {
                            format!(
                                "Could not create url from file path '{}'",
                                file_with_extension.display()
                            )
                        })?;

                    return Ok(file_path_as_url.to_string());
                }
            }
        }

        bail!(
            "No file found at path '{}' with any of these extensions '{:?}'",
            file.display(),
            extensions
        )
    }
}

#[cfg(test)]
mod test {
    use std::ffi::OsStr;
    use std::path::Path;

    use super::super::provider::Provider;
    use super::FileProvider;

    #[test]
    fn get_default_sample() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("Could not get CARGO_MANIFEST_DIR");
        let path = root.join("samples/hello-world");
        match FileProvider::find_file(&path, "context", &["toml"]) {
            Ok(path_string) => {
                let path = Path::new(&path_string);
                assert_eq!(Some(OsStr::new("context.toml")), path.file_name());
            }
            _ => panic!("Could not find_file 'context.toml'"),
        }
    }

    #[test]
    fn resolve_url_file_not_found() {
        let provider: &dyn Provider = &FileProvider;
        let url = "file://directory";
        let _ = provider.resolve_url(url, "default", &["toml"]);
    }

    #[test]
    fn get_contents_file_not_found() {
        let provider: &dyn Provider = &FileProvider;
        assert!(provider.get_contents("file:///no-such-file").is_err());
    }
}
