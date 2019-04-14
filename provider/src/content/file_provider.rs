use std::fs::File;
use std::fs::metadata;
use std::io;
use std::io::ErrorKind;
use std::io::prelude::*;
use std::path::PathBuf;

use flowrlib::provider::Provider;
use glob::glob;
use url::Url;

pub struct FileProvider;

impl Provider for FileProvider {
    fn resolve(&self, url_str: &str, default_filename: &str) -> Result<(String, Option<String>), String> {
        let url = Url::parse(url_str)
            .map_err(|_| format!("Could not convert '{}' to Url", url_str))?;
        let mut path = url.to_file_path().unwrap();
        match metadata(&path) {
            Ok(md) => {
                if md.is_dir() {
                    info!("'{}' is a directory, so attempting to find context file in it",
                          path.display());
                    let file = FileProvider::find_default_file(&mut path, default_filename).
                        map_err(|e| e.to_string())?;
                    let resolved_url = Url::from_file_path(&file)
                        .map_err(|_| format!("Could not create url from file path '{}'",
                                             file.to_str().unwrap()))?;
                    Ok((resolved_url.into_string(), None))
                } else {
                    Ok((url.to_string(), None))
                }
            }
            Err(e) => {
                Err(format!("Error getting file metadata for path: '{}', {}", path.display(), e))
            }
        }
    }

    fn get(&self, url_str: &str) -> Result<Vec<u8>, String> {
        let url = Url::parse(url_str)
            .map_err(|_| format!("Could not convert '{}' to Url", url_str))?;
        let file_path = url.to_file_path().unwrap();
        let mut f = File::open(&file_path)
            .map_err(|e| format!("Could not open file '{:?}' ({}", file_path, e))?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)
            .map_err(|e| format!("Could not read content from '{:?}' ({}", file_path, e))?;
        Ok(buffer)
    }
}

impl FileProvider {
    /*
        Passed a path to a directory, it searches for a file in the directory called 'default_filename'
        If found, it opens the file and returns its contents as a String in the result
    */
    fn find_default_file(path: &mut PathBuf, default_filename: &str) -> io::Result<PathBuf> {
        // TODO pending more complex patterns based on implemented loaders
        // Or iterate through the matches until a loader is found which understands that file extension
        path.push(default_filename);
        let pattern = path.to_str().unwrap();
        info!("Looking for files matching: '{}'", pattern);

        // Try to glob for the default file using a pattern
        for entry in glob(pattern).expect("Failed to read glob pattern") {
            // return first file found that matches the pattern, or error if none match
            match entry {
                Ok(context_file) => return Ok(context_file),
                Err(_) => return Err(io::Error::new(ErrorKind::NotFound,
                                                    format!("No context file found matching '{}'",
                                                            path.display())))
            }
        }

        // No matches
        Err(io::Error::new(ErrorKind::NotFound,
                           format!("No default context file found. Tried '{}'", path.display())))
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
        match FileProvider::find_default_file(&mut path, "context.toml") {
            Ok(path) => {
                if path.file_name().unwrap() != "context.toml" {
                    assert!(false);
                }
            }
            _ => assert!(false),
        }
    }

    #[test]
    #[should_panic]
    fn get_contents_file_not_found() {
        let provider: &Provider = &FileProvider;
        provider.get("file:///no-such-file").unwrap();
    }
}
