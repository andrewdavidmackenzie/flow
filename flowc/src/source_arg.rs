use std::fs;
use std::path::PathBuf;

use tempdir::TempDir;
use url::Url;

use crate::errors::*;

/*
    Determine the output directory to use for generation on the local file system as a
    function of the url of the source flow, and the optional argument to specify the output
    directory to use.
    The flow source location can be http url, or file url
*/
pub fn get_output_dir(url: &Url, option: Option<&str>) -> Result<PathBuf> {
    let mut output_dir;

    // Allow the optional command line argument to force output_dir
    if let Some(dir) = option {
        output_dir = PathBuf::from(dir);
    } else {
        match url.scheme() {
            // If loading flow from a local file, then generate in the same directory
            "file" => {
                let dir = url.to_file_path()
                    .map_err(|_| format!("Error converting url to file path\nurl = '{}'", url))?;
                output_dir = dir;
                if output_dir.is_file() {
                    output_dir.pop(); // remove trailing filename
                }
            }
            // If not from a file, then create a dir with flow name under a temp dir
            _ => {
                let dir = TempDir::new("flow")
                    .chain_err(|| "Error creating new TempDir".to_string())?;
                output_dir = dir.into_path();
            }
        }
    }

    Ok(make_writeable(output_dir)?)
}

fn make_writeable(output_dir: PathBuf) -> Result<PathBuf> {
    // Now make sure the directory exists, if not create it, and is writable
    if output_dir.exists() {
        let md = fs::metadata(&output_dir)
            .chain_err(|| format!("Could not read metadata of the existing output directory '{}'",
                                  output_dir.to_str().unwrap()))?;
        // Check it's not a file!
        if md.is_file() {
            bail!("Output directory '{}' already exists as a file", output_dir.to_str().unwrap());
        }

        // check it's not read only!
        if md.permissions().readonly() {
            bail!("Output directory '{}' is read only", output_dir.to_str().unwrap());
        }
    } else {
        fs::create_dir(&output_dir).chain_err(|| format!("Could not create directory '{}'",
                                                         output_dir.to_str().unwrap()))?;
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
    fn file_url_no_output_dir_arg() {
        let temp_dir = TempDir::new("flow").unwrap().into_path();
        let flow_dir = temp_dir.to_str().unwrap();
        let flow_path = format!("{}/fake.toml", flow_dir);
        let mut file = fs::File::create(&flow_path).unwrap();
        file.write_all(b"flow = 'test'").unwrap();
        let url = Url::parse(&format!("file://{}", flow_path)).unwrap();

        let dir = super::get_output_dir(&url, None).unwrap();

        assert_eq!(dir.to_str().unwrap(), flow_dir);
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
