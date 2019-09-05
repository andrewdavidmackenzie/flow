use std::fs::File;
use std::fs::metadata;
use std::io;
use std::io::ErrorKind;
use std::io::prelude::*;
use std::path::PathBuf;

use url::Url;

use flowrlib::errors::*;
use flowrlib::provider::Provider;
use glob::glob;

pub struct FileProvider;

impl Provider for FileProvider {
    fn resolve(&self, url_str: &str, default_filename: &str) -> Result<(String, Option<String>)> {
        let url = Url::parse(url_str)
            .chain_err(|| format!("Could not convert '{}' to Url", url_str))?;
        let mut path = url.to_file_path().unwrap();
        let md = metadata(&path)
            .chain_err(|| format!("Error getting file metadata for path: '{}'", path.display()))? ;

        if md.is_dir() {
            info!("'{}' is a directory, so attempting to find default file named '{}' in it",
                  path.display(), default_filename);
            let file = FileProvider::find_default_file(&mut path, default_filename).
                chain_err(|| format!("Could not find default file called '{}'", default_filename))?;
            let resolved_url = Url::from_file_path(&file)
                .map_err(|_| format!("Could not create url from file path '{}'",
                                      file.to_str().unwrap()))?;
            Ok((resolved_url.into_string(), None))
        } else {
            Ok((url.to_string(), None))
        }
    }

    fn get(&self, url_str: &str) -> Result<Vec<u8>> {
        let url = Url::parse(url_str)
            .chain_err(|| format!("Could not convert '{}' to Url", url_str))?;
        let file_path = url.to_file_path().unwrap();
        let mut f = File::open(&file_path)
            .chain_err(|| format!("Could not open file '{:?}'", file_path))?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)
            .chain_err(|| format!("Could not read content from '{:?}'", file_path))?;
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
        let provider: &dyn Provider = &FileProvider;
        provider.get("file:///no-such-file").unwrap();
    }
}
