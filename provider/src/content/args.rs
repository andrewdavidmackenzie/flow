use std::env;

use url::{ParseError, Url};

/*
    Use the current working directory as the starting point ("parent") for parsing a command
    line specified url where to load the flow from. This allows specifiying of full Urls
    (http, file etc) as well as file paths relative to the working directory.

    Returns a full url with appropriate scheme, and an absolute path.

    From the (optional) Command Line argument for url or filename of a flow, create an
    absolute path url with scheme to try and load the flow from:
        - if no parameter was passed --> use parent
        - if parameter passed then join to parent, which will inherit the scheme of none is
          specified, and will resolve relative path if passed
*/
pub fn url_from_string(string: Option<&str>) -> Result<Url, String> {
    let parent = cwd_as_url()?;

    match string {
        None => {
            info!("No url specified, so using: '{}'", parent);
            Ok(parent.clone())
        }
        Some(url_string) => {
            parent.join(url_string).map_err(|e: ParseError| e.to_string())
        }
    }
}

pub fn cwd_as_url() -> Result<Url, String> {
    Url::from_directory_path(env::current_dir().unwrap())
        .map_err(|_e| "Could not form a Url for the current working directory".to_string())
}

#[cfg(test)]
mod test {
    extern crate url;

    use std::fs;
    use std::io::Write;
    use std::path;

    use tempdir::TempDir;
    use url::Url;

    use super::cwd_as_url;
    use super::url_from_string;

// Tests for url_from_cl_arg

    #[test]
    fn no_arg_returns_parent() {
        let url = url_from_string(None).unwrap();

        let cwd = cwd_as_url();
        assert_eq!(url, cwd);
    }

    #[test]
    fn file_scheme_in_arg_absolute_path_preserved() {
        let path = "/some/file";
        let arg = format!("file:{}", path);

        let url = url_from_string(Some(&arg)).unwrap();

        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), path);
    }

    #[test]
    fn http_scheme_in_arg_absolute_path_preserved() {
        let path = "/some/file";
        let arg = format!("http://test.com{}", path);

        let url = url_from_string(Some(&arg)).unwrap();

        assert_eq!(url.scheme(), "http");
        assert_eq!(url.path(), path);
    }

    #[test]
    fn no_scheme_in_arg_assumes_file() {
        let arg = "/some/file";

        let url = url_from_string(Some(arg)).unwrap();

        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), arg);
    }

    #[test]
    fn relative_path_in_arg_converted_to_absolute_path_and_scheme_added() {
        // Get the path of this file relative to project root (where Cargo.toml is)
        let relative_path_to_file = file!();
        let dir = path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let url = url_from_string(Some(&relative_path_to_file)).unwrap();

        let abs_path = format!("{}/{}", &dir.display(), relative_path_to_file);
        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), abs_path);
    }
}
