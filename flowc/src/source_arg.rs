use url::{Url, ParseError};
use std::env;
use std::path::PathBuf;
use tempdir::TempDir;
use std::fs;

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
pub fn url_from_cl_arg(cl_arg: Option<&str>) -> Result<Url, String> {
    let parent = cwd_as_url();

    match cl_arg {
        None => {
            info!("No url specified, so using: '{}'", parent);
            Ok(parent.clone())
        }
        Some(cl_url_string) => {
            parent.join(cl_url_string).map_err(|e: ParseError| e.to_string())
        }
    }
}

fn cwd_as_url() -> Url {
    Url::from_directory_path(env::current_dir().unwrap()).unwrap()
}

/*
    Determine the output directory to use for generation on the local file system as a
    function of the url of the source flow, and the optional argument to specify the output
    directory to use.
    The flow source location can be http url, or file url
*/
pub fn get_output_dir(url: &Url, option: Option<&str>) -> Result<PathBuf, String> {
    let mut output_dir;

    // Allow the optional command line argument to force output_dir
    if let Some(dir) = option {
        output_dir = PathBuf::from(dir);
    } else {
        match url.scheme() {
            // If loading flow from a local file, then generate in the same directory
            "file" => {
                let dir = url.to_file_path()
                    .map_err(|_e| format!("Error converting url to file path\nurl = '{}'", url))?;
                output_dir = dir.clone();
                if output_dir.is_file() {
                    output_dir.pop(); // remove trailing filename
                }
                output_dir.push("rust"); // add rust directory alongside input file
            }
            // If not from a file, then create a dir with flow name under a temp dir
            _ => {
                let dir = TempDir::new("flow")
                    .map_err(|e| format!("Error creating new TempDir, \n'{}'",
                                         e.to_string()))?;
                output_dir = dir.into_path();
            }
        }
    }

    Ok(make_writeable(output_dir)?)
}

fn make_writeable(output_dir: PathBuf) -> Result<PathBuf, String> {
    // Now make sure the directory exists, if not create it, and is writable
    if output_dir.exists() {
        let md = fs::metadata(&output_dir).map_err(|e| e.to_string())?;
        // Check it's not a file!
        if md.is_file() {
            return Err(format!("Output directory '{}' already exists as a file",
                               output_dir.to_str().unwrap()));
        }

        // check it's not read only!
        if md.permissions().readonly() {
            return Err(format!("Output directory '{}' is read only", output_dir.to_str().unwrap()));
        }
    } else {
        fs::create_dir(&output_dir).map_err(|e| e.to_string())?;
    }

    Ok(output_dir)
}

#[cfg(test)]
mod test {
    extern crate url;

    use url::Url;
    use std::path;
    use tempdir::TempDir;
    use std::fs;
    use std::io::Write;

    use super::url_from_cl_arg;
    use super::cwd_as_url;

    // Tests for url_from_cl_arg

    #[test]
    fn no_arg_returns_parent() {
        let url = url_from_cl_arg(None).unwrap();

        let cwd = cwd_as_url();
        assert_eq!(url, cwd);
    }

    #[test]
    fn file_scheme_in_arg_absolute_path_preserved() {
        let path = "/some/file";
        let arg = format!("file:{}", path);

        let url = url_from_cl_arg(Some(&arg)).unwrap();

        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), path);
    }

    #[test]
    fn http_scheme_in_arg_absolute_path_preserved() {
        let path = "/some/file";
        let arg = format!("http://test.com{}", path);

        let url = url_from_cl_arg(Some(&arg)).unwrap();

        assert_eq!(url.scheme(), "http");
        assert_eq!(url.path(), path);
    }

    #[test]
    fn no_scheme_in_arg_assumes_file() {
        let arg = "/some/file";

        let url = url_from_cl_arg(Some(arg)).unwrap();

        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), arg);
    }

    #[test]
    fn relative_path_in_arg_converted_to_absolute_path_and_scheme_added() {
        // Get the path of this file relative to project root (where Cargo.toml is)
        let relative_path_to_file = file!();
        let dir = path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let url = url_from_cl_arg(Some(&relative_path_to_file)).unwrap();

        let abs_path = format!("{}/{}", &dir.display(), relative_path_to_file);
        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), abs_path);
    }

    // Tests for get_output_dir, after the url for flow has been determined

    #[test]
    fn http_url_no_output_dir_arg() {
        let url = &Url::parse("http://test.com/dir/file.flow").unwrap();

        let dir = super::get_output_dir(url, None);

        assert!(dir.unwrap().exists());
    }

    #[test]
    fn http_with_output_dir_arg() {
        let url = &Url::parse("http://test.com/dir/file.flow").unwrap();

        let temp_dir = TempDir::new("flow").unwrap().into_path();
        let out_dir_arg = temp_dir.to_str().unwrap();

        let dir = super::get_output_dir(url, Some(&out_dir_arg));

        assert_eq!(dir.unwrap().to_str().unwrap(), out_dir_arg);
    }

    #[test]
    fn output_dir_is_created() {
        let url = &Url::parse("http://test.com/dir/file.flow").unwrap();

        let temp_dir = TempDir::new("flow").unwrap().into_path();
        let out_dir_arg = format!("{}/subdir", temp_dir.to_str().unwrap());

        let dir = super::get_output_dir(url, Some(&out_dir_arg))
            .unwrap();

        assert_eq!(dir.to_str().unwrap(), out_dir_arg);
        assert!(dir.exists());
    }

    #[test]
    #[ignore]
    fn file_url_no_output_dir_arg() {
        let temp_dir = TempDir::new("flow").unwrap().into_path();
        let flow_dir = temp_dir.to_str().unwrap();
        let flow_path = format!("{}/fake.toml", flow_dir);
        let mut file = fs::File::create(&flow_path).unwrap();
        file.write_all(b"flow = 'test'").unwrap();
        let url = Url::parse(&format!("file:/{}", flow_path)).unwrap();
        println!("flow_url = {}", url);

        let dir = super::get_output_dir(&url, None)
            .unwrap();

        assert_eq!(dir.to_str().unwrap(), format!("file:/{}/rust", flow_dir));
        assert!(dir.exists());
    }

    #[test]
    fn file_url_output_dir_arg() {
        // FLow url
        let temp_dir = TempDir::new("flow").unwrap().into_path();
        let flow_dir = temp_dir.to_str().unwrap();
        let flow_path = format!("{}/fake.toml", flow_dir);
        let url = Url::parse(&format!("file:/{}", flow_path)).unwrap();

        // Output dir arg
        let temp_dir = TempDir::new("flow").unwrap().into_path();
        let out_dir_arg = temp_dir.to_str().unwrap();

        let dir = super::get_output_dir(&url, Some(&out_dir_arg))
            .unwrap();

        assert_eq!(dir.to_str().unwrap(), out_dir_arg);
        assert!(dir.exists());
    }
}
