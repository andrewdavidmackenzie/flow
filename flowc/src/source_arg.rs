extern crate url;
use url::{Url, ParseError};

/*
    From the (optional) Command Line argument for url or filename of a flow, create an
    absolute path url with scheme to try and load the flow from:
        - if no parameter was passed --> use parent
        - if parameter passed then join to parent, which will inherit the scheme of none is
          specified, and will resolve relative path if passed
*/
pub fn url_from_cl_arg(parent: &Url, cl_arg: Option<&str>) -> Result<Url, String> {
    match cl_arg {
        None => {
            info!("No url specified, so using parent: '{}'", parent);
            Ok(parent.clone())
        },
        Some(cl_url_string) => {
            parent.join(cl_url_string).map_err(|e: ParseError|
                e.to_string())
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

    fn cwd_as_url() -> Url {
        Url::from_directory_path(env::current_dir().unwrap()).unwrap()
    }

    /*
        If URL has a scheme, then it must be absolute path.
        If URL does not have a scheme, then inherit it from parent
    */
    #[test]
    fn no_option_returns_parent() {
        let parent = cwd_as_url();

        let url = url_from_cl_arg(&parent,None).unwrap();

        assert_eq!(url, parent);
    }

    #[test]
    fn file_scheme_and_absolute_path_preserved() {
        let path = "/some/file";
        let parent = cwd_as_url();

        let url = url_from_cl_arg(&parent,Some(&format!("file:{}", path))).unwrap();

        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), path);
    }

    #[test]
    fn http_scheme_and_absolute_path_preserved() {
        let path = "/some/file";
        let arg = format!("http://test.com{}", path);
        let parent = cwd_as_url();

        let url = url_from_cl_arg(&parent,Some(&arg)).unwrap();

        assert_eq!(url.scheme(), "http");
        assert_eq!(url.path(), path);
    }

    #[test]
    fn no_scheme_assumes_file() {
        let path = "/some/file";

        let url = url_from_cl_arg(&cwd_as_url(),Some(path)).unwrap();

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

    #[test]
    fn relative_path_from_http_parent() {
        let path = "/some/file.flow";
        let parent = Url::parse(&format!("http://test.com{}", path)).unwrap();
        println!("parent = {}", parent);
        let relative_path = "other_file.flow";

        let url = url_from_cl_arg(&parent, Some(&relative_path)).unwrap();

        assert_eq!(url.scheme(), "http");
        assert_eq!(url.path(), "/some/other_file.flow");
    }

    #[test]
    fn absolute_path_from_http_parent() {
        let path = "/some/file.flow";
        let parent = Url::parse(&format!("http://test.com{}", path)).unwrap();
        println!("parent = {}", parent);
        let new_path = "/other/file.flow";

        let url = url_from_cl_arg(&parent, Some(&new_path)).unwrap();

        assert_eq!(url.scheme(), "http");
        assert_eq!(url.path(), "/other/file.flow");
    }
}
