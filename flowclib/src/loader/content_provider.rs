use url::Url;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::io::prelude::*;
use glob::glob;
use std::io;
use std::io::ErrorKind;
use std::fs::metadata;

/*
    Accept a Url that:
     -  maybe url formatted with http/https, file, or lib
     That could point to:
     -  a specific file, that may or may not exist, try to open it
     -  a specific directory, that may or may not exist

     If no file is specified, then look for default file in a directory specified
*/
pub fn find(url: Url) -> Result<Url, String>{
    // TODO Implement switch by scheme - for now assume file:
    get_file(url)
}

fn get_file(url: Url) -> Result<Url, String> {
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
            Err(format!("Error getting file metadata for path: '{}', {}", path.display(), e))
        }
    }
}

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

// Helper method to read the content of a file found at 'file_path' into a String result.
// 'file_path' could be absolute or relative, so we canonicalize it first...
pub fn get_contents(url: &Url) -> Result<String, String> {
    // TODO extend this to load definition from a URI that is a file, url or a lib: reference...
    match url.scheme() {
        "file" => {
            let file_path = url.to_file_path().unwrap();
            get_file_contents(&file_path)
        }
        _ => Err(format!("Loading from '{}' scheme not implemented yet", url.scheme()))
    }
}

fn get_file_contents(file_path: &PathBuf) -> Result<String, String> {
    match File::open(file_path) {
        Ok(file) => {
            let mut buf_reader = BufReader::new(file);
            let mut contents = String::new();

            match buf_reader.read_to_string(&mut contents) {
                Ok(_) => Ok(contents),
                Err(e) => Err(format!("{}", e))
            }
        }
        Err(e) => Err(format!("{}", e))
    }
}

#[cfg(test)]
mod test {
    use url::Url;
    use super::get_contents;

    #[test]
    #[should_panic]
    fn get_contents_file_not_found() {
        get_contents(&Url::parse("file:///no-such-file").unwrap()).unwrap();
    }
}
