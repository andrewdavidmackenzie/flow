use url::Url;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::io::prelude::*;
use glob::glob;
use std::io;
use std::io::ErrorKind;
use std::fs::metadata;
use content::provider::Provider;

pub struct FileProvider;

impl Provider for FileProvider {
    fn find(&self, url: &Url) -> Result<Url, String> {
        let mut path = url.to_file_path().unwrap();
        match metadata(&path) {
            Ok(md) => {
                if md.is_dir() {
                    info!("'{}' is a directory, so attempting to find context file in it",
                          path.display());
                    let file = FileProvider::find_default_file(&mut path).
                        map_err(|e| e.to_string())?;
                    Url::from_file_path(&file)
                        .map_err(|_| format!("Could not create url from file path '{}'",
                                              file.to_str().unwrap()))
                } else {
                    Ok(url.clone())
                }
            }
            Err(e) => {
                Err(format!("Error getting file metadata for path: '{}', {}", path.display(), e))
            }
        }
    }

    fn get(&self, url: &Url) -> Result<String, String> {
        let file_path = url.to_file_path().unwrap();
        match File::open(file_path) {
            Ok(file) => {
                let mut buf_reader = BufReader::new(file);
                let mut contents = String::new();

                match buf_reader.read_to_string(&mut contents) {
                    Ok(_) => Ok(contents),
                    Err(e) => Err(format!("{}", e))
                }
            }
            Err(e) => Err(format!("Could not load content from URL '{}' ({}", url, e))
        }
    }
}

impl FileProvider {
    /*
    Passed a path to a directory, it searches for the first file it can find in the directory
    fitting the pattern "context.*", and if found opens it and returns it in the result
    */
    fn find_default_file(path: &mut PathBuf) -> io::Result<PathBuf> {
        path.push("context.*");
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
                           format!("No default context file found in directory '{}'", path.display())))
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;
    use url::Url;
    use super::FileProvider;
    use content::provider::Provider;

    #[test]
    fn get_default_sample() {
        let mut path = PathBuf::from("../samples/hello-world");
        match FileProvider::find_default_file(&mut path) {
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
        provider.get(&Url::parse("file:///no-such-file").unwrap()).unwrap();
    }
}
