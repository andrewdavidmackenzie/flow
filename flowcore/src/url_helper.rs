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
        Some(url_string) => {
            // Detect absolute file paths and convert to file:// URLs.
            // Without this, Url::join interprets drive letters (C:, D:) as
            // URL schemes on Windows.
            #[cfg(not(target_arch = "wasm32"))]
            {
                let path = std::path::Path::new(url_string);
                if path.is_absolute() {
                    return Url::from_file_path(path).map_err(|()| {
                        format!("Could not convert path '{url_string}' to file URL").into()
                    });
                }
            }
            if is_windows_absolute_path(url_string) {
                let normalized = url_string.replace('\\', "/");
                return Url::parse(&format!("file:///{normalized}")).chain_err(|| {
                    format!("Could not convert Windows path '{url_string}' to file URL")
                });
            }
            base_url
                .join(url_string)
                .chain_err(|| format!("Problem joining url '{base_url}' with '{url_string}'"))
        }
    }
}

fn is_windows_absolute_path(s: &str) -> bool {
    let bytes = s.as_bytes();
    if let [drive, b':', sep, ..] = bytes {
        drive.is_ascii_alphabetic() && (*sep == b'\\' || *sep == b'/')
    } else {
        false
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
        let arg = "some/relative/file";

        let url = url_from_string(&cwd, Some(arg)).expect("Could not form URL");

        assert_eq!(url.scheme(), "file");
        assert!(
            url.path().ends_with(arg),
            "URL path '{}' should end with '{arg}'",
            url.path()
        );
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

    #[test]
    fn is_windows_path_detected() {
        assert!(super::is_windows_absolute_path("C:\\Users\\test"));
        assert!(super::is_windows_absolute_path("D:/some/path"));
        assert!(super::is_windows_absolute_path("c:\\file"));
        assert!(!super::is_windows_absolute_path("/unix/path"));
        assert!(!super::is_windows_absolute_path("relative/path"));
        assert!(!super::is_windows_absolute_path("http://example.com"));
        assert!(!super::is_windows_absolute_path("file:///test"));
        assert!(!super::is_windows_absolute_path("C:"));
    }

    #[test]
    fn windows_backslash_path_becomes_file_url() {
        let cwd = cwd_as_url();
        let win_path = "C:\\Users\\test\\myflow";
        let url = url_from_string(&cwd, Some(win_path)).expect("Could not form URL");
        assert_eq!(url.scheme(), "file");
        assert!(
            url.path().contains("Users/test/myflow"),
            "URL path '{}' should contain the path segments",
            url.path()
        );
    }

    #[test]
    fn windows_forward_slash_path_becomes_file_url() {
        let cwd = cwd_as_url();
        let win_path = "D:/a/flow/flow/examples";
        let url = url_from_string(&cwd, Some(win_path)).expect("Could not form URL");
        assert_eq!(url.scheme(), "file");
        assert!(
            url.path().contains("a/flow/flow/examples"),
            "URL path '{}' should contain the path segments",
            url.path()
        );
    }

    #[cfg(unix)]
    #[test]
    fn unix_absolute_path_becomes_file_url() {
        let cwd = cwd_as_url();
        let url = url_from_string(&cwd, Some("/tmp/myflow")).expect("Could not form URL");
        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), "/tmp/myflow");
    }

    #[cfg(windows)]
    #[test]
    fn windows_native_absolute_path_becomes_file_url() {
        let cwd = cwd_as_url();
        let url =
            url_from_string(&cwd, Some("C:\\Users\\test\\myflow")).expect("Could not form URL");
        assert_eq!(url.scheme(), "file");
        assert!(url.path().contains("Users/test/myflow"));
    }

    #[test]
    fn canonicalized_path_to_directory_url_round_trips() {
        let dir = std::env::temp_dir()
            .canonicalize()
            .expect("Could not canonicalize temp dir");
        let url = Url::from_directory_path(&dir).unwrap_or_else(|()| {
            panic!(
                "Url::from_directory_path failed for canonicalized path: {}",
                dir.display()
            )
        });
        assert_eq!(url.scheme(), "file");
        let back = url
            .to_file_path()
            .unwrap_or_else(|()| panic!("to_file_path failed for URL: {url}"));
        assert_eq!(back, dir, "round-trip failed: {dir:?} -> {url} -> {back:?}");
    }

    #[test]
    fn canonicalized_path_to_file_url_round_trips() {
        let file = std::env::temp_dir().join("test_round_trip.txt");
        std::fs::write(&file, "test").expect("Could not write test file");
        let canonical = file
            .canonicalize()
            .expect("Could not canonicalize test file");
        let url = Url::from_file_path(&canonical).unwrap_or_else(|()| {
            panic!(
                "Url::from_file_path failed for canonicalized path: {}",
                canonical.display()
            )
        });
        assert_eq!(url.scheme(), "file");
        let back = url
            .to_file_path()
            .unwrap_or_else(|()| panic!("to_file_path failed for URL: {url}"));
        assert_eq!(
            back, canonical,
            "round-trip failed: {canonical:?} -> {url} -> {back:?}"
        );
        let _ = std::fs::remove_file(&file);
    }

    #[cfg(windows)]
    #[test]
    fn windows_unc_path_to_url_round_trips() {
        let cwd = std::env::current_dir().expect("Could not get cwd");
        let canonical = cwd.canonicalize().expect("Could not canonicalize cwd");
        let canonical_str = canonical.to_string_lossy();
        eprintln!("Canonical path: {canonical_str}");
        assert!(
            canonical_str.starts_with("\\\\?\\"),
            "Expected UNC prefix on Windows canonicalize, got: {canonical_str}"
        );
        let url = Url::from_directory_path(&canonical).unwrap_or_else(|()| {
            panic!("Url::from_directory_path failed for UNC path: {canonical_str}")
        });
        eprintln!("URL: {url}");
        let joined = url
            .join("manifest.json")
            .expect("Could not join manifest.json");
        eprintln!("Joined URL: {joined}");
        let back = joined
            .to_file_path()
            .unwrap_or_else(|()| panic!("to_file_path failed for joined URL: {joined}"));
        eprintln!("Back to path: {}", back.display());
        assert!(
            back.ends_with("manifest.json"),
            "Path should end with manifest.json, got: {}",
            back.display()
        );
    }

    #[test]
    fn canonicalize_then_set_cwd_then_get_cwd() {
        let original_cwd = std::env::current_dir().expect("Could not get cwd");
        let dir = std::env::temp_dir();
        let canonical = dir.canonicalize().expect("Could not canonicalize temp dir");
        eprintln!("canonical temp dir: {}", canonical.display());
        std::env::set_current_dir(&canonical).expect("Could not set cwd to canonical path");
        let retrieved_cwd = std::env::current_dir().expect("Could not get cwd after set");
        eprintln!("retrieved cwd: {}", retrieved_cwd.display());
        let url = Url::from_directory_path(&retrieved_cwd).unwrap_or_else(|()| {
            panic!(
                "Url::from_directory_path failed for retrieved cwd: {}",
                retrieved_cwd.display()
            )
        });
        eprintln!("url from retrieved cwd: {url}");
        let back = url
            .to_file_path()
            .unwrap_or_else(|()| panic!("to_file_path failed for: {url}"));
        eprintln!("back to path: {}", back.display());
        std::env::set_current_dir(&original_cwd).expect("Could not restore cwd");
    }

    #[test]
    fn canonicalized_cwd_produces_valid_url_for_manifest() {
        let cwd = std::env::current_dir()
            .expect("Could not get cwd")
            .canonicalize()
            .expect("Could not canonicalize cwd");
        let cwd_url = Url::from_directory_path(&cwd).unwrap_or_else(|()| {
            panic!(
                "Url::from_directory_path failed for canonicalized cwd: {}",
                cwd.display()
            )
        });
        let manifest_url = cwd_url
            .join("manifest.json")
            .expect("Could not join manifest.json to cwd url");
        assert!(
            manifest_url.as_str().ends_with("manifest.json"),
            "manifest URL should end with manifest.json, got: {manifest_url}"
        );
        let manifest_path = manifest_url
            .to_file_path()
            .expect("Could not convert manifest URL back to path");
        assert!(
            manifest_path.ends_with("manifest.json"),
            "manifest path should end with manifest.json, got: {}",
            manifest_path.display()
        );
    }
}
