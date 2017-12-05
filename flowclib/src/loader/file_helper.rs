use std::fs::File;
use std::fs;
use std::io::BufReader;
use std::path::PathBuf;
use std::io::prelude::*;

// Helper method to read the content of a file found at 'file_path' into a String result.
// 'file_path' could be absolute or relative, so we canonicalize it first...
pub fn get_contents(file_path: &PathBuf) -> Result<String, String> {
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

#[test]
#[should_panic]
fn get_contents_file_not_found() {
    get_contents(&PathBuf::from("no-such-file")).unwrap();
}

// NOTE: these unwraps fail if the files don't actually exist!
// NOTE: seems to me like canonicalize does not guarantee an absolute path
pub fn get_canonical_path(parent_path: PathBuf, child_path: PathBuf) -> PathBuf {
    if child_path.is_relative() {
        fs::canonicalize(parent_path).unwrap().parent().unwrap().join(child_path)
    } else {
        child_path
    }
}

#[test]
fn absolute_path() {
    let path = get_canonical_path(PathBuf::from("/root/me/original_file"),
                                  PathBuf::from("/users/home/my_file"));
    assert_eq!(path.to_str().unwrap(), "/users/home/my_file");
}

// TODO test for relative path that always works...
