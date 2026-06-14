use log::info;
use url::Url;

use crate::errors::{Result, ResultExt};

/// `url_from_string` accepts an optional string (URL or filename) and creates an absolute path URL
///
/// This allows specifying of full URL (http, file etc) as well as file paths relative
/// to the working directory.
///
/// Depending on the parameter passed in:
/// - None                --> Return the Current Working Directory (CWD)
/// - Some(absolute path) --> Return the absolute path passed in
/// - Some(relative path) --> Join the CWD with the relative path and return the resulting
///   `  `                          absolute path.
///
/// Returns a full URL with appropriate scheme (depending on the original scheme passed in),
/// and an absolute path.
///
/// # Errors
///
/// Returns an error if a new `Url` cannot be formed by joining `string` to the end of `base_url`
///
pub fn url_from_string(base_url: &Url, string: Option<&str>) -> Result<Url> {
    match string {
        None => {
            info!("No url specified, so using: '{base_url}'");
            Ok(base_url.clone())
        }
        Some(url_string) => base_url
            .join(url_string)
            .chain_err(|| format!("Problem joining url '{base_url}' with '{url_string}'")),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
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
        // Use a real temp dir for a valid absolute path on all platforms
        let tmp = std::env::temp_dir().join("test_file");
        let tmp_url = Url::from_file_path(&tmp).expect("Could not form URL");
        let arg = tmp_url.as_str().to_string();

        let url = url_from_string(&cwd, Some(&arg)).expect("Could not form URL");

        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), tmp_url.path());
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
        // Use a real absolute path that works on all platforms
        let tmp = std::env::temp_dir().join("test_file");
        let tmp_str = tmp.to_str().expect("Could not convert to str");
        let expected_url = Url::from_file_path(&tmp).expect("Could not form URL");

        let url = url_from_string(&cwd, Some(tmp_str)).expect("Could not form URL");

        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), expected_url.path());
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
        let abs_path = root.join(relative_path_to_file);
        let expected_url = Url::from_file_path(&abs_path).expect("Could not form URL");

        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), expected_url.path());
    }
}
