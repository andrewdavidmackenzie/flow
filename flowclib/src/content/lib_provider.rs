use url::Url;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::io::prelude::*;
use content::provider::Provider;

pub struct LibProvider;

/*
    Urls for library flows and functions and values will be of the form:
    source = "lib://flowstdlib/src/stdio/stdout.toml"
    Where 'flowstdlib' is the library name and 'src/stdio/stdout.toml' the path of the definition file
    within the library.

*/
impl Provider for LibProvider {
    /*
        For the lib provider, find will just confirm the file at the specified url exists
    */
    fn find(&self, url: &Url) -> Result<Url, String> {
        LibProvider::path_from_lib_url(url).map(|_path| url.clone())
    }

    fn get(&self, url: &Url) -> Result<String, String> {
        let file_path = LibProvider::path_from_lib_url(url)?;
        match File::open(file_path) {
            Ok(file) => {
                let mut buf_reader = BufReader::new(file);
                let mut contents = String::new();

                match buf_reader.read_to_string(&mut contents) {
                    Ok(_) => Ok(contents),
                    Err(e) => Err(format!("{}", e))
                }
            }
            Err(e) => Err(format!("Could not load content from url '{}' ({}", url, e))
        }
    }
}

impl LibProvider {
    // Take the lib url and convert to a path where the corresponding definition files
    // should be in the local install, below where this binary is running from
    fn path_from_lib_url(url: &Url) -> Result<PathBuf, String> {
        let mut path = Self::lib_root();
        path.push(url.host_str().unwrap());
        path.push("src"); // TODO
        let lib_path = &url.path()[1..]; // Strip off leading '/'
        path.push(lib_path);

        if path.exists() {
            Ok(path)
        } else {
            Err(format!("Could not locate url '{}' in installed libraries", url))
        }
    }

    fn lib_root() -> PathBuf {
        const LIB_ROOT: &'static str = "/Users/andrew/workspace/flow";

        PathBuf::from(LIB_ROOT)
    }
}

#[cfg(test)]
mod test {
    use url::Url;
    use super::LibProvider;
    use content::provider::Provider;

    #[test]
    #[should_panic]
    fn get_contents_file_not_found() {
        let provider: &Provider = &LibProvider;
        provider.get(&Url::parse("lib:///no-such-file").unwrap()).unwrap();
    }
}
