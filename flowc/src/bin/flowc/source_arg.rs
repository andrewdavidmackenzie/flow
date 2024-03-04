use std::{env, fs};
use std::path::{Path, PathBuf};

use tempfile::tempdir;
use url::Url;

use crate::errors::*;
use crate::RunnerSpec;

pub(crate) enum CompileType {
    Library,
    Flow,
    Runner(String),
}

fn default_lib_compile_dir(source_url: &Url) -> Result<PathBuf> {
    let lib_name = source_url.path_segments()
        .ok_or("Could not get path of source_url")?.last()
        .ok_or("Could not get last path segment of source_url")?;

    let home_dir = env::var("HOME").expect("Could not get $HOME");

    Ok(PathBuf::from(format!("{}/.flow/lib/{}", home_dir, lib_name)))
}
pub(crate) fn default_runner_dir(runner_name: String) -> Result<PathBuf> {
    let home_dir = env::var("HOME").expect("Could not get $HOME");
    Ok(PathBuf::from(format!("{}/.flow/runner/{}", home_dir, runner_name)))
}

// Load a `RunnerSpec` from the context at `context_root`
pub(crate) fn load_runner_spec(path: &Path) -> Result<RunnerSpec> {
    let runner_spec = fs::read_to_string(path)?;
    Ok(toml::from_str(&runner_spec)?)
}

fn default_flow_compile_dir(source_url: &Url) -> Result<PathBuf> {
    let mut output_dir;

    match source_url.scheme() {
        // If loading flow from a local file, then build in the same directory
        "file" => {
            output_dir = source_url
                .to_file_path()
                .map_err(|_| format!("Error converting url to file path\nurl = '{source_url}'"))?;
            if output_dir.is_file() {
                output_dir.pop(); // remove trailing filename
            }
        }
        // If not from a file, then create a dir with flow name under a temp dir
        _ => {
            let dir = tempdir().chain_err(|| "Error creating new tempdir".to_string())?;
            output_dir = dir.into_path();
        }
    }

    Ok(output_dir)
}

/// Determine the output directory to use for generation on the local file system as a
/// function of the url of the source flow, and the optional argument to specify the output
/// directory to use.
/// The flow source location can be http url, or file url
pub(crate) fn get_output_dir(source_url: &Url, option: &Option<String>, compile_type: CompileType) -> Result<PathBuf> {
    let mut output_dir;

    // Allow the optional command line argument to force output_dir
    if let Some(dir) = option {
        output_dir = PathBuf::from(dir);
        if output_dir.is_relative() {
            output_dir = env::current_dir()?.join(output_dir);
        }
        if output_dir.is_file() {
            output_dir.pop(); // remove trailing filename
        }
    } else {
        match compile_type {
            CompileType::Library => output_dir = default_lib_compile_dir(source_url)?,
            CompileType::Flow => output_dir = default_flow_compile_dir(source_url)?,
            CompileType::Runner(name) => output_dir = default_runner_dir(name)?,
        }
    }

    Ok(output_dir)
}

#[cfg(test)]
mod test {
    use std::fs;
    use std::io::Write;

    use tempfile::tempdir;
    use url::Url;

    use crate::source_arg::CompileType;

// Tests for get_output_dir, after the url for flow has been determined

    #[test]
    fn http_url_no_output_dir_arg() {
        let url = &Url::parse("http://test.com/dir/file.flow").expect("Could not parse test url");

        let dir = super::get_output_dir(url, &None, CompileType::Flow)
            .expect("Could not get output dir");

        assert!(dir.exists());
    }

    #[test]
    fn http_with_output_dir_arg() {
        let url = &Url::parse("http://test.com/dir/file.flow").expect("Could not parse test url");

        let temp_dir = tempdir()
            .expect("Could not create temporary directory for test")
            .into_path();
        let out_dir_arg = temp_dir
            .to_str()
            .expect("Could not convert temp dir to String");

        let dir = super::get_output_dir(url, &Some(out_dir_arg.to_string()),
                                        CompileType::Flow)
            .expect("Could not get output dir");

        assert_eq!(
            dir.to_str().expect("Could not convert dir ot String"),
            out_dir_arg
        );
    }

    #[test]
    fn file_url_no_output_dir_arg() {
        let temp_dir =tempdir()
            .expect("Could not create temporary directory for test")
            .into_path();
        let flow_dir = temp_dir
            .to_str()
            .expect("Could not convert temp dir name to string");
        let flow_path = format!("{flow_dir}/fake.toml");
        let mut file = fs::File::create(&flow_path).expect("Could not create file");
        file.write_all(b"flow = 'test'")
            .expect("Could not write to file");
        let url = Url::parse(&format!("file://{flow_path}"))
            .expect("Could not parse test Url");

        let dir = super::get_output_dir(&url, &None, CompileType::Flow)
            .expect("Could not get output dir");

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
        let temp_dir = tempdir()
            .expect("Could not create temporary directory for test")
            .into_path();
        let flow_dir = temp_dir
            .to_str()
            .expect("Could not convert temp dir name to string");

        let flow_path = format!("{flow_dir}/fake.toml");
        let url = Url::parse(&format!("file:/{flow_path}")).expect("Could not parse test Url");

        // Output dir arg
        let temp_dir = tempdir()
            .expect("Could not create temporary directory for test")
            .into_path();
        let out_dir_arg = temp_dir
            .to_str()
            .expect("Could not convert temp dir name to string");

        let dir =
            super::get_output_dir(&url, &Some(out_dir_arg.to_string()), CompileType::Flow)
                .expect("Could not get output dir");

        assert_eq!(
            dir.to_str().expect("Could not convert dir ot String"),
            out_dir_arg
        );
        assert!(dir.exists());
    }
}
