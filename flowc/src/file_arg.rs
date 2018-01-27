extern crate url;
use url::{Url, ParseError};

/*
    From the (optional) Command Line argument for url or filename of a flow, create an
    absolute path url with scheme to try and load the flow from:
        - if no parameter was passed --> use parent
        - if parameter passed then join to parent, which will inherit the scheme of none is
          specified, and will resolve relative path if passed
*/
pub fn url_from_cl_arg(parent: &Url, cl_arg: Option<&str>) -> Result<Url, ParseError> {
    match cl_arg {
        None => {
            info!("No url specified, so using parent: '{}'", parent);
            Ok(parent.clone())
        },
        Some(cl_url_string) => {
            parent.join(cl_url_string)
        }
    }
}

#[cfg(test)]
mod test {
    extern crate url;
    use url::Url;
    use std::env;
    use std::path;

    use super::url_from_cl_arg;

    /*
        If URL has a scheme, then it must be absolute path.
        If URL does not have a scheme, then we assume it's a file and it could relative to CWD
    */
    #[test]
    fn no_option_returns_parent() {
        let parent = Url::from_directory_path(env::current_dir().unwrap()).unwrap();

        let url = url_from_cl_arg(&parent,None).unwrap();

        assert_eq!(url, parent);
    }

    #[test]
    fn file_scheme_and_absolute_path_preserved() {
        let path = "/some/file";
        let parent = Url::from_directory_path(env::current_dir().unwrap()).unwrap();

        let url = url_from_cl_arg(&parent,Some(&format!("file:{}", path))).unwrap();

        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), path);
    }

    #[test]
    fn no_scheme_assumes_file() {
        let parent = Url::from_directory_path(env::current_dir().unwrap()).unwrap();
        let path = "/some/file";

        let url = url_from_cl_arg(&parent,Some(path)).unwrap();

        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), path);
    }

    #[test]
    fn relative_path_converted_to_absolute_scheme_added() {
        // Get the path of this file relative to project root (where Cargo.toml is)
        let this_file = file!();
        let dir = path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let parent = Url::from_directory_path(&dir).unwrap();

        let url = url_from_cl_arg(&parent, Some(&this_file)).unwrap();

        let abs_path = format!("{}/{}", &dir.display(), this_file);
        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), abs_path);
    }
}
