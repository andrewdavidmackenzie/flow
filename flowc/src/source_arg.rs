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
            info!("No url specified, so using parent: '{}'", parent);
            Ok(parent.clone())
        }
        Some(cl_url_string) => {
            parent.join(cl_url_string).map_err(|e: ParseError|
                e.to_string())
        }
    }
}

fn cwd_as_url() -> Url {
    Url::from_directory_path(env::current_dir().unwrap()).unwrap()
}

pub fn get_output_dir(url: &Url, option: Option<&str>) -> Result<PathBuf, String> {
    let mut output_dir;

    // Allow the optional command line argument to force output_dir
    if let Some(dir) = option {
        output_dir = PathBuf::from(dir);
    } else {
        match url.scheme() {
            // If loading flow from a local file, then generate in the same directory
            "file" => {
                output_dir = url.to_file_path().unwrap().clone();
                output_dir.pop();
                output_dir.push("rust");
            }
            // If not from a file, then create a dir with flow name under a temp dir
            _ => {
                output_dir = TempDir::new("flow").unwrap().into_path();
            }
        }
    }

    println!("directory = {}", output_dir.to_str().unwrap());

    // Now make sure the directory exists, if not create it, and is writable
    if output_dir.exists() {
        // Check it's not a file!
        let md = fs::metadata(&output_dir).map_err(|e| e.to_string())?;
        if md.is_file() {
            return Err(format!("Output directory '{}' already exists as a file",
                        output_dir.to_str().unwrap()));
        }

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

    use super::url_from_cl_arg;
    use super::cwd_as_url;

    /*
        If URL has a scheme, then it must be absolute path.
        If URL does not have a scheme, then inherit it from parent
    */
    #[test]
    fn no_option_returns_parent() {
        let parent = cwd_as_url();

        let url = url_from_cl_arg(None).unwrap();

        assert_eq!(url, parent);
    }

    #[test]
    fn file_scheme_and_absolute_path_preserved() {
        let path = "/some/file";

        let url = url_from_cl_arg(Some(&format!("file:{}", path))).unwrap();

        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), path);
    }

    #[test]
    fn http_scheme_and_absolute_path_preserved() {
        let path = "/some/file";
        let arg = format!("http://test.com{}", path);

        let url = url_from_cl_arg(Some(&arg)).unwrap();

        assert_eq!(url.scheme(), "http");
        assert_eq!(url.path(), path);
    }

    #[test]
    fn no_scheme_assumes_file() {
        let path = "/some/file";

        let url = url_from_cl_arg(Some(path)).unwrap();

        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), path);
    }

    #[test]
    fn relative_path_converted_to_absolute_scheme_added() {
        // Get the path of this file relative to project root (where Cargo.toml is)
        let this_file = file!();
        let dir = path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let url = url_from_cl_arg(Some(&this_file)).unwrap();

        let abs_path = format!("{}/{}", &dir.display(), this_file);
        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), abs_path);
    }

    #[test]
    fn http_temp_dir() {
        let dir = super::get_output_dir(&Url::parse("http://test.com/dir/file.flow").unwrap(), None);

        assert!(dir.unwrap().exists());
    }

    #[test]
    fn output_dir_arg() {
        let temp_dir = TempDir::new("flow").unwrap().into_path();
        let out_dir_arg = temp_dir.to_str().unwrap();

        let dir = super::get_output_dir(
            &Url::parse("http://test.com/dir/file.flow").unwrap(),
            Some(&out_dir_arg));

        assert_eq!(dir.unwrap().to_str().unwrap(), out_dir_arg);
    }

    #[test]
    fn output_dir_created() {
        let temp_dir = TempDir::new("flow").unwrap().into_path();
        let out_dir_arg = format!("{}/subdir", temp_dir.to_str().unwrap());

        let dir = super::get_output_dir(
            &Url::parse("http://test.com/dir/file.flow").unwrap(),
            Some(&out_dir_arg));

        assert_eq!(dir.unwrap().to_str().unwrap(), out_dir_arg);
    }

    #[test]
    fn file_same_directory() {
        let temp_dir = TempDir::new("flow").unwrap().into_path();
        let flow_dir = temp_dir.to_str().unwrap();
        let flow_loc = format!("{}/fake.flow", flow_dir);
        let flow_url = Url::parse(&format!("file://{}", flow_loc)).unwrap();

        let dir = super::get_output_dir(&flow_url, None);

        assert_eq!(dir.unwrap().to_str().unwrap(), format!("{}/rust", flow_dir));
    }
}
