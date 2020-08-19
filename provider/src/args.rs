//! Help take file/url strings from a command line and convert them
//! into URLs (as Strings) with schemes for use with flowlibc and flowlibr.
use std::env;

use log::info;
use url::Url;

use crate::errors::*;

/// Accept an optional string (URL or filename) and from it create an absolute path URL with correct
/// scheme. This allows specifiying of full URL (http, file etc) as well as file paths relative
/// to the working directory.
///
/// Depending on the parameter passed in:
/// - None                --> Return the Current Working Directory (CWD)
/// - Some(absolute path) --> Return the absolute path passed in
/// - Some(relative path) --> Join the CWD with the relative path and return the resulting
///                           absolute path.
///
/// Returns a full URL with appropriate scheme (depending on the original scheme passed in),
/// and an absolute path.
///
pub fn url_from_string(base_url: &Url, string: Option<&str>) -> Result<Url> {
    match string {
        None => {
            info!("No url specified, so using: '{}'", base_url);
            Ok(base_url.clone())
        }
        Some(url_string) => {
            base_url.join(url_string)
                .chain_err(|| format!("Problem joining url '{}' with '{}'", base_url, url_string))
        }
    }
}

///
/// Provide the Current Working Directory (CWD) as a URL (with 'file:' scheme) or an error if it cannot be found.
///
pub fn cwd_as_url() -> Result<Url> {
    Url::from_directory_path(
        env::current_dir().chain_err(|| "Could not get current working directory value")?)
        .map_err(|_| "Could not form a Url for the current working directory".into())
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;
    use std::str::FromStr;

    use url::Url;

    use super::cwd_as_url;
    use super::url_from_string;

    #[test]
    fn no_arg_returns_parent() {
        let cwd = cwd_as_url().unwrap();
        let url = url_from_string(&cwd, None).unwrap();

        assert_eq!(url, cwd);
    }

    #[test]
    fn file_scheme_in_arg_absolute_path_preserved() {
        let cwd = cwd_as_url().unwrap();
        let path = "/some/file";
        let arg = format!("file:{}", path);

        let url = url_from_string(&cwd, Some(&arg)).unwrap();

        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), path);
    }

    #[test]
    fn http_scheme_in_arg_absolute_path_preserved() {
        let cwd = cwd_as_url().unwrap();
        let path = "/some/file";
        let arg = format!("http://test.com{}", path);

        let url = url_from_string(&cwd, Some(&arg)).unwrap();

        assert_eq!(url.scheme(), "http");
        assert_eq!(url.path(), path);
    }

    #[test]
    fn no_scheme_in_arg_assumes_file() {
        let cwd = cwd_as_url().unwrap();
        let arg = "/some/file";

        let url = url_from_string(&cwd, Some(arg)).unwrap();

        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), arg);
    }

    fn check_flow_root() {
        if std::env::var("FLOW_ROOT").is_err() {
            println!("FLOW_ROOT environment variable must be set for testing. Set it to the root\
            directory of the project and ensure it has a trailing '/'");
            std::process::exit(1);
        }
    }

    #[test]
    fn relative_path_in_arg_converted_to_absolute_path_and_scheme_added() {
        check_flow_root();

        let root = PathBuf::from_str(&std::env::var("FLOW_ROOT").unwrap()).unwrap();
        let root_url = Url::from_directory_path(&root).unwrap();

        // the path of this file relative to project root
        let relative_path_to_file = "provider/src/args.rs";

        let url = url_from_string(&root_url, Some(&relative_path_to_file)).unwrap();

        let abs_path = format!("{}{}", root.display(), relative_path_to_file);
        println!("abs_path = {}", abs_path);
        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), abs_path);
    }
}
