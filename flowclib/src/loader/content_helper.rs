use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::io::prelude::*;
use url::Url;

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
