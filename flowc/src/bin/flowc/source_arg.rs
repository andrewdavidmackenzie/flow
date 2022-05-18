use std::env;
use std::path::PathBuf;

use tempdir::TempDir;
use url::Url;

use crate::errors::*;

/// Determine the output directory to use for generation on the local file system as a
/// function of the url of the source flow, and the optional argument to specify the output
/// directory to use.
/// The flow source location can be http url, or file url
pub fn get_output_dir(url: &Url, option: Option<&str>) -> Result<PathBuf> {
    let mut output_dir;

    // Allow the optional command line argument to force output_dir
    if let Some(dir) = option {
        output_dir = PathBuf::from(dir);
        if output_dir.is_relative() {
            output_dir = env::current_dir()?.join(output_dir);
        }
    } else {
        match url.scheme() {
            // If loading flow from a local file, then generate in the same directory
            "file" => {
                let dir = url
                    .to_file_path()
                    .map_err(|_| format!("Error converting url to file path\nurl = '{}'", url))?;
                output_dir = dir;
                if output_dir.is_file() {
                    output_dir.pop(); // remove trailing filename
                }
            }
            // If not from a file, then create a dir with flow name under a temp dir
            _ => {
                let dir =
                    TempDir::new("flow").chain_err(|| "Error creating new TempDir".to_string())?;
                output_dir = dir.into_path();
            }
        }
    }

    Ok(output_dir)
}

#[cfg(test)]
mod test {
    use std::fs;
    use std::io::Write;

    use tempdir::TempDir;
    use url::Url;

// Tests for get_output_dir, after the url for flow has been determined

    #[test]
    fn http_url_no_output_dir_arg() {
        let url = &Url::parse("http://test.com/dir/file.flow").expect("Could not parse test url");

        let dir = super::get_output_dir(url, None).expect("Could not get output dir");

        assert!(dir.exists());
    }

    #[test]
    fn http_with_output_dir_arg() {
        let url = &Url::parse("http://test.com/dir/file.flow").expect("Could not parse test url");

        let temp_dir = TempDir::new("flow")
            .expect("Could not create TempDir for test")
            .into_path();
        let out_dir_arg = temp_dir
            .to_str()
            .expect("Could not convert temp dir to String");

        let dir = super::get_output_dir(url, Some(out_dir_arg)).expect("Could not get output dir");

        assert_eq!(
            dir.to_str().expect("Could not convert dir ot String"),
            out_dir_arg
        );
    }

    #[test]
    fn file_url_no_output_dir_arg() {
        let temp_dir = TempDir::new("flow")
            .expect("Could not create TempDir for test")
            .into_path();
        let flow_dir = temp_dir
            .to_str()
            .expect("Could not convert temp dir name to string");
        let flow_path = format!("{}/fake.toml", flow_dir);
        let mut file = fs::File::create(&flow_path).expect("Could not create file");
        file.write_all(b"flow = 'test'")
            .expect("Could not write to file");
        let url = Url::parse(&format!("file://{}", flow_path)).expect("Could not parse test Url");

        let dir = super::get_output_dir(&url, None).expect("Could not get output dir");

        assert_eq!(
            dir.to_str()
                .expect("Could not convert output directory to file"),
            flow_dir
        );
        assert!(dir.exists());
    }

    #[test]
    fn file_url_output_dir_arg() {
        // FLow url
        let temp_dir = TempDir::new("flow")
            .expect("Could not create TempDir for test")
            .into_path();
        let flow_dir = temp_dir
            .to_str()
            .expect("Could not convert temp dir name to string");

        let flow_path = format!("{}/fake.toml", flow_dir);
        let url = Url::parse(&format!("file:/{}", flow_path)).expect("Could not parse test Url");

        // Output dir arg
        let temp_dir = TempDir::new("flow")
            .expect("Could not create TempDir for test")
            .into_path();
        let out_dir_arg = temp_dir
            .to_str()
            .expect("Could not convert temp dir name to string");

        let dir =
            super::get_output_dir(&url, Some(out_dir_arg)).expect("Could not get output dir");

        assert_eq!(
            dir.to_str().expect("Could not convert dir ot String"),
            out_dir_arg
        );
        assert!(dir.exists());
    }
}
