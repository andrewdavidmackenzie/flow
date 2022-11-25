use log::info;
use url::Url;

use crate::errors::*;

/// Accept an optional string (URL or filename) and from it create an absolute path URL with correct
/// scheme. This allows specifying of full URL (http, file etc) as well as file paths relative
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
        Some(url_string) => base_url
            .join(url_string)
            .chain_err(|| format!("Problem joining url '{base_url}' with '{url_string}'")),
    }
}

#[cfg(test)]
mod test {
    use std::env;
    use std::path::PathBuf;
    use std::str::FromStr;

    use url::Url;

    use crate::url_helper::url_from_string;

    fn cwd_as_url() -> Url {
        Url::from_directory_path(env::current_dir().expect("Couldn't get CWD"))
            .expect("Could not convert CWD to a URL")
    }

    #[test]
    fn no_arg_returns_parent() {
        let cwd = cwd_as_url();
        let url = url_from_string(&cwd, None).expect("Could not form URL");

        assert_eq!(url, cwd);
    }

    #[test]
    fn file_scheme_in_arg_absolute_path_preserved() {
        let cwd = cwd_as_url();
        let path = "/some/file";
        let arg = format!("file:{path}");

        let url = url_from_string(&cwd, Some(&arg)).expect("Could not form URL");

        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), path);
    }

    #[test]
    fn http_scheme_in_arg_absolute_path_preserved() {
        let cwd = cwd_as_url();
        let path = "/some/file";
        let arg = format!("http://test.com{path}");

        let url = url_from_string(&cwd, Some(&arg)).expect("Could not form URL");

        assert_eq!(url.scheme(), "http");
        assert_eq!(url.path(), path);
    }

    #[test]
    fn no_scheme_in_arg_assumes_file() {
        let cwd = cwd_as_url();
        let arg = "/some/file";

        let url = url_from_string(&cwd, Some(arg)).expect("Could not form URL");

        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), arg);
    }

    #[test]
    fn relative_path_in_arg_converted_to_absolute_path_and_scheme_added() {
        let mut root = PathBuf::from_str(env!("CARGO_MANIFEST_DIR"))
            .expect("Could not get CARGO_MANIFEST_DIR");
        root.pop();
        let root_url = Url::from_directory_path(&root).expect("Could not form URL");

        // the path of this file relative to crate root
        let relative_path_to_file = "src/url";

        let url =
            url_from_string(&root_url, Some(relative_path_to_file)).expect("Could not form URL");
        let abs_path = format!("{}/{relative_path_to_file}", root.display());

        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), abs_path);
    }
}
