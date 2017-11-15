use glob::glob;
use std::io;
use std::io::ErrorKind;
use std::fs::File;
use std::fs::metadata;
use std::path::PathBuf;

/*
    Passed a path to a directory, it searches for the first file it can find in the directory
    fitting the pattern "*.context", and if found opens it and returns it in the result
*/
fn open_default_file(path: PathBuf) -> io::Result<File> {
    let file_pattern = format!("{}/*.context", path.display());
    info!("Looking for default files matching: '{}' in path '{}'", file_pattern, path.display());

    // Try to glob for the default file using a pattern
    for entry in glob(file_pattern.as_str()).expect("Failed to read glob pattern") {
        // return first file found that matches the pattern, or error if none match
        // TODO this by just checking size of paths and accessing first entry?
        match entry {
            Ok(context_file) => return File::open(context_file),
            Err(_) => return Err(io::Error::new(ErrorKind::NotFound,
                                         format!("No default context file found in directory '{}'", path.display())))
        }
    }

    Err(io::Error::new(ErrorKind::NotFound,
                       format!("No default context file found in directory '{}'", path.display())))
}

/*
    Accept a path that could point to:
     -  a specific file, that may or may not exist, try to open it
     -  a specific directory, that may or may not exist,
        look for default file type in it by extension
*/
pub fn open(path: PathBuf) -> io::Result<File> {
    match metadata(&path) {
        Ok(md) => {
            if md.is_dir() {
                open_default_file(path)
            } else {
                File::open(path)
            }
        },
        Err(e) => {
            debug!("Error getting file metadata for path: '{}', {}", path.display(), e);
            Err(io::Error::new(ErrorKind::NotFound,
                               format!("File or Directory '{}' could not be found or opened. ({})", path.display(), e)))
        }
    }
}