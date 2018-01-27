use glob::glob;
use std::io;
use std::io::ErrorKind;
use std::fs::metadata;
use std::path::PathBuf;

extern crate url;
use url::Url;

/*
    Passed a path to a directory, it searches for the first file it can find in the directory
    fitting the pattern "context.*", and if found opens it and returns it in the result

    TODO for http/https this will have to work differently, looking for each valid option in turn
*/
fn get_default_file(path: PathBuf) -> io::Result<PathBuf> {
    let file_pattern = format!("{}/context.*", path.display());
    info!("Looking for files matching: '{}'", file_pattern);

    // Try to glob for the default file using a pattern
    for entry in glob(file_pattern.as_str()).expect("Failed to read glob pattern") {
        // return first file found that matches the pattern, or error if none match
        match entry {
            Ok(context_file) => return Ok(context_file),
            Err(_) => return Err(io::Error::new(ErrorKind::NotFound,
                                         format!("No default context file found in directory '{}'", path.display())))
        }
    }

    Err(io::Error::new(ErrorKind::NotFound,
                       format!("No default context file found in directory '{}'", path.display())))
}

#[test]
fn get_default_sample() {
    let path = PathBuf::from("../samples/hello-world");
    match get_default_file(path) {
        Ok(path) => {
            if path.file_name().unwrap() != "context.toml" {
                assert!(false);
            }
        },
        _ => assert!(false),
    }
}

/*
    Accept a Url that:
     -  maybe url formatted with http/https, file, or lib
     That could point to:
     -  a specific file, that may or may not exist, try to open it
     -  a specific directory, that may or may not exist

     If no file is specified, then look for default file in a directory specified
*/
pub fn find(url: Url) -> io::Result<Url> {
    info!("Attempting to open flow at url = '{}'", url);

    // TODO Implement switch by scheme - for now assume file:
    get_file(url)
}

fn get_file(url: Url) -> io::Result<Url> {
    let path = url.to_file_path().unwrap();
    match metadata(&path) {
        Ok(md) => {
            if md.is_dir() {
                info!("'{}' is a directory, so attempting to find context file in it", path.display());
                // TODO see how to handle conversion of error types if these fail...
                Ok(Url::from_file_path(get_default_file(path).unwrap()).unwrap())
            } else {
                Ok(url)
            }
        },
        Err(e) => {
            error!("Error getting file metadata for path: '{}', {}", path.display(), e);
            Err(io::Error::new(ErrorKind::NotFound,
                               format!("File or Directory '{}' could not be found or opened. ({})", path.display(), e)))
        }
    }
}