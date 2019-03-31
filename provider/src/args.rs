//! Help take file/url strings from a command line and convert them
//! into URLs (as Strings) with schemes for use with flowlibc and flowlibr.
use std::env;

use url::Url;

/// Accept an optional string (URL or filename) and from it create an absolute path URL with correct
/// scheme. This allows specifiying of full URL (http, file etc) as well as file paths relative
/// to the working directory.
///
/// Depending on the parameter passed in:
/// - no parameter passed     --> Return the Current Working Directory (CWD)
/// - absolute path passed in --> Return the absolute path passed in
/// - relative path passed in --> Join the CWD with the relative path and return the resulting
///                               absolute path.
///
/// Returns a full URL with appropriate scheme (depending on the original scheme passed in),
/// and an absolute path.
///
pub fn url_from_string(string: Option<&str>) -> Result<Url, String> {
    let parent = cwd_as_url()?;

    match string {
        None => {
            info!("No url specified, so using: '{}'", parent);
            Ok(parent.clone())
        }
        Some(url_string) => {
            parent.join(url_string).map_err(|_|
                format!("Problem joining url '{}' with '{}'", parent, url_string))
        }
    }
}

///
/// Provide the Current Working Directory (CWD) as a URL (with 'file:' scheme) or an error message
/// String if it cannot be found.
///
pub fn cwd_as_url() -> Result<Url, String> {
    Url::from_directory_path(env::current_dir().unwrap())
        .map_err(|_e| "Could not form a Url for the current working directory".to_string())
}

#[cfg(test)]
mod test {
    extern crate url;

    use std::path;

    use super::cwd_as_url;
    use super::url_from_string;

// Tests for url_from_cl_arg

    #[test]
    fn no_arg_returns_parent() {
        let url = url_from_string(None).unwrap();

        let cwd = cwd_as_url().unwrap();
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
