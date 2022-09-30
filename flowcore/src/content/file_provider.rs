use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use log::{debug, trace};
use url::Url;

use crate::errors::*;
use crate::provider::Provider;

/// The `FileProvider` implements the `Provider` trait and takes care of fetching content located
/// on the local file system.
pub struct FileProvider;

impl Provider for FileProvider {
    fn resolve_url(
        &self,
        url: &Url,
        default_filename: &str,
        extensions: &[&str],
    ) -> Result<(Url, Option<Url>)> {
        let path = url
            .to_file_path()
            .map_err(|_| format!("Could not convert '{}' to a file path", url))?;
        let md_result = fs::metadata(&path)
            .chain_err(|| format!("Error getting file metadata for path: '{}'", path.display()));

        match md_result {
            Ok(md) => {
                if md.is_dir() {
                    trace!(
                        "'{}' is a directory, so attempting to find default file named '{}' in it",
                        path.display(),
                        default_filename
                    );
                    if let Ok(file_found_url) =
                        FileProvider::find_file(&path, default_filename, extensions)
                    {
                        return Ok((file_found_url, None));
                    }

                    let dir_os_name = path.file_name().ok_or("Could not get directory name")?;
                    let dir_name = dir_os_name.to_string_lossy();
                        trace!(
                            "'{}' is a directory, so attempting to find file named '{}' inside it",
                            path.display(), dir_name);
                        if let Ok(file_found_url) = Self::find_file(&path, &dir_name, extensions) {
                            return Ok((file_found_url, None));
                    }

                    bail!("No file named '{}' or '{}' with extension '{}' found in directory '{}'",
                        default_filename, dir_name, extensions.join(" or "), path.display())
                } else if md.is_file() {
                    Ok((url.clone(), None))
                } else {
                    let file_found_url = Self::file_by_extensions(&path, extensions)?;
                    Ok((file_found_url, None))
                }
            }
            _ => {
                // as-is the file at path doesn't exist, try with extensions
                let file_found_url = Self::file_by_extensions(&path, extensions)?;
                Ok((file_found_url, None))
            }
        }
    }

    fn get_contents(&self, url: &Url) -> Result<Vec<u8>> {
        let file_path = url
            .to_file_path()
            .map_err(|_| "Could not convert Url to file path")?;
        let mut f =
            File::open(&file_path).map_err(|_| format!("Could not open file '{:?}'", file_path))?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)
            .chain_err(|| format!("Could not read content from '{:?}'", file_path))?;
        Ok(buffer)
    }
}

impl FileProvider {
    // Passed a path to a directory, it searches for a file in the directory called 'default_filename'
    // If found, it opens the file and returns its contents as a String in the result
    fn find_file(dir: &Path, default_filename: &str, extensions: &[&str]) -> Result<Url> {
        let mut file = dir.to_path_buf();
        file.push(default_filename);

        Self::file_by_extensions(&file, extensions)
    }

    // Given a path to a filename, try to find an existing file with any of the allowed extensions
    fn file_by_extensions(file: &Path, extensions: &[&str]) -> Result<Url> {
        let mut file_with_extension = file.to_path_buf();

        // for that file path, try with all the allowed file extensions
        for extension in extensions {
            file_with_extension.set_extension(extension);
            debug!("Looking for file '{}'", file_with_extension.display());
            if let Ok(md) = fs::metadata(&file_with_extension) {
                if md.is_file() {
                    let file_path_as_url =
                        Url::from_file_path(&file_with_extension).map_err(|_| {
                            format!(
                                "Could not create url from file path '{}'",
                                file_with_extension.display()
                            )
                        })?;

                    return Ok(file_path_as_url);
                }
            }
        }

        bail!(
            "No file found at path '{}' with any of these extensions '{:?}'",
            file.display(),
            extensions
        )
    }
}

#[cfg(test)]
mod test {
    mod file_provider {
        use std::ffi::OsStr;
        use std::path::Path;

        use super::super::FileProvider;

        #[test]
        fn get_default_sample() {
            let root = Path::new(env!("CARGO_MANIFEST_DIR"));
            let path = root.join("tests/test-flows/hello-world");
            match FileProvider::find_file(&path, "root", &["toml"]) {
                Ok(path_string) => {
                    let path = path_string
                        .to_file_path()
                        .expect("Could not convert Url to File Path");
                    assert_eq!(Some(OsStr::new("root.toml")), path.file_name());
                }
                _ => panic!("Could not find_file 'root.toml'"),
            }
        }
    }

    mod provider_trait {
        use std::path::Path;

        use url::Url;

        use crate::provider::Provider;

        use super::super::FileProvider;

        #[test]
        fn get_default_sample_full_path() {
            let root = Path::new(env!("CARGO_MANIFEST_DIR"));
            let path = root.join("tests/test-flows/hello-world/root.toml");
            let url = Url::from_file_path(path).expect("Could not create Url from path");
            let provider: &dyn Provider = &FileProvider;
            let resolved_url = provider
                .resolve_url(&url, "root", &["toml"])
                .expect("Could not resolve url");
            assert_eq!(resolved_url.0, url);

            let _ = provider
                .get_contents(&resolved_url.0)
                .expect("Could not fetch contents");
        }

        #[test]
        fn get_default_sample_full_path_without_extension() {
            let root = Path::new(env!("CARGO_MANIFEST_DIR"));
            let path = root.join("tests/test-flows/hello-world/root");
            let url = Url::from_file_path(path).expect("Could not create Url from path");
            let provider: &dyn Provider = &FileProvider;
            let resolved_url = provider
                .resolve_url(&url, "root", &["toml"])
                .expect("Could not resolve url");
            assert_eq!(
                resolved_url.0,
                url.join("root.toml").expect("Could not join")
            );

            let _ = provider
                .get_contents(&resolved_url.0)
                .expect("Could not fetch contents");
        }

        #[test]
        fn get_default_sample_rom_dir() {
            let root = Path::new(env!("CARGO_MANIFEST_DIR"));
            let path = root.join("tests/test-flows/hello-world");
            let url = Url::from_file_path(path.clone()).expect("Could not create Url from path");
            let provider: &dyn Provider = &FileProvider;
            let resolved_url = provider
                .resolve_url(&url, "root", &["toml"])
                .expect("Could not resolve url");
            let expected_url = Url::from_file_path(path.join("root.toml"))
                .expect("Could not create Url from path");
            assert_eq!(resolved_url.0, expected_url);

            let _ = provider
                .get_contents(&resolved_url.0)
                .expect("Could not fetch contents");
        }

        #[test]
        fn get_by_last_path_segment() {
            let root = Path::new(env!("CARGO_MANIFEST_DIR"));
            let path = root.join("tests/test-flows/control/compare_switch");
            let url = Url::from_file_path(path.clone()).expect("Could not create Url from path");
            let provider: &dyn Provider = &FileProvider;
            let resolved_url = provider
                .resolve_url(&url, "root", &["toml"])
                .expect("Could not resolve url");
            let expected_url = Url::from_file_path(path.join("compare_switch.toml"))
                .expect("Could not create Url from path");
            assert_eq!(resolved_url.0, expected_url);

            let _ = provider
                .get_contents(&resolved_url.0)
                .expect("Could not fetch contents");
        }

        #[test]
        fn resolve_url_file_not_found() {
            let provider: &dyn Provider = &FileProvider;
            let url = Url::parse("file://directory").expect("Could not create Url");
            let _ = provider.resolve_url(&url, "default", &["toml"]);
        }

        #[test]
        fn get_contents_file_not_found() {
            let provider: &dyn Provider = &FileProvider;
            let url = Url::parse("file:///no-such-file").expect("Could not create Url");
            assert!(provider.get_contents(&url).is_err());
        }
    }
}
