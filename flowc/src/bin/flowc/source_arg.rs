use std::path::{Path, PathBuf};
use std::{env, fs};

use tempfile::tempdir;
use url::Url;

use crate::errors::{Result, ResultExt};
use crate::RunnerSpec;

pub(crate) enum CompileType {
    Library,
    Flow,
    Runner(String),
}

fn default_lib_compile_dir(source_url: &Url) -> Result<PathBuf> {
    let lib_name = source_url
        .path_segments()
        .ok_or("Could not get path of source_url")?
        .next_back()
        .ok_or("Could not get last path segment of source_url")?;

    flowcore::dirs::lib_dir()
        .map(|d| d.join(lib_name))
        .ok_or_else(|| "Could not determine flow data directory".into())
}

pub(crate) fn default_runner_dir(runner_name: &str) -> Result<PathBuf> {
    // Standard data directory location
    if let Some(dir) = flowcore::dirs::runner_dir(runner_name) {
        if dir.is_dir() {
            return Ok(dir);
        }
    }

    // Fallback: next to the binary (for portable installs)
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let system_dir = exe_dir.join("runner").join(runner_name);
            if system_dir.is_dir() {
                return Ok(system_dir);
            }
        }
    }

    // Return the standard path even if it doesn't exist yet (it may be created)
    flowcore::dirs::runner_dir(runner_name)
        .ok_or_else(|| "Could not determine flow data directory".into())
}

// Load a `RunnerSpec` from the context at `context_root`
pub(crate) fn load_runner_spec(path: &Path) -> Result<RunnerSpec> {
    let runner_spec = fs::read_to_string(path)?;
    Ok(toml::from_str(&runner_spec)?)
}

fn default_flow_compile_dir(source_url: &Url) -> Result<PathBuf> {
    let mut output_dir;

    #[allow(clippy::single_match_else)]
    match source_url.scheme() {
        // If loading flow from a local file, then build in the same directory
        "file" => {
            output_dir = source_url
                .to_file_path()
                .map_err(|()| format!("Error converting url to file path\nurl = '{source_url}'"))?;
            if output_dir.is_file() {
                output_dir.pop(); // remove trailing filename
            }
        }
        // If not from a file, then create a dir with flow name under a temp dir
        _ => {
            let dir = tempdir().chain_err(|| "Error creating new tempdir".to_string())?;
            output_dir = dir.keep();
        }
    }

    Ok(output_dir)
}

/// Determine the output directory to use for generation on the local file system as a
/// function of the url of the source flow, and the optional argument to specify the output
/// directory to use.
/// The flow source location can be http url, or file url
#[allow(clippy::ref_option)]
pub(crate) fn get_output_dir(
    source_url: &Url,
    option: &Option<String>,
    compile_type: CompileType,
) -> Result<PathBuf> {
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
            CompileType::Runner(name) => output_dir = default_runner_dir(&name)?,
        }
    }

    Ok(output_dir)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use std::fs;
    use std::io::Write;

    use tempfile::tempdir;
    use url::Url;

    use crate::source_arg::CompileType;

    // Tests for get_output_dir, after the url for flow has been determined

    #[test]
    fn http_url_no_output_dir_arg() {
        let url = &Url::parse("https://test.com/dir/file.flow").expect("Could not parse test url");

        let dir =
            super::get_output_dir(url, &None, CompileType::Flow).expect("Could not get output dir");

        assert!(dir.exists());
    }

    #[test]
    fn http_with_output_dir_arg() {
        let url = &Url::parse("https://test.com/dir/file.flow").expect("Could not parse test url");

        let temp_dir = tempdir()
            .expect("Could not create temporary directory for test")
            .keep();
        let out_dir_arg = temp_dir
            .to_str()
            .expect("Could not convert temp dir to String");

        let dir = super::get_output_dir(url, &Some(out_dir_arg.to_string()), CompileType::Flow)
            .expect("Could not get output dir");

        assert_eq!(
            dir.to_str().expect("Could not convert dir ot String"),
            out_dir_arg
        );
    }

    #[test]
    fn file_url_no_output_dir_arg() {
        let temp_dir = tempdir()
            .expect("Could not create temporary directory for test")
            .keep();
        let flow_path = temp_dir.join("fake.toml");
        let mut file = fs::File::create(&flow_path).expect("Could not create file");
        file.write_all(b"flow = 'test'")
            .expect("Could not write to file");
        let url =
            Url::from_file_path(&flow_path).expect("Could not create Url from flow file path");

        let dir = super::get_output_dir(&url, &None, CompileType::Flow)
            .expect("Could not get output dir");

        assert_eq!(dir, temp_dir);
        assert!(dir.exists());
    }

    #[test]
    fn file_url_output_dir_arg() {
        let temp_dir = tempdir()
            .expect("Could not create temporary directory for test")
            .keep();
        let flow_path = temp_dir.join("fake.toml");
        let url =
            Url::from_file_path(&flow_path).expect("Could not create Url from flow file path");

        // Output dir arg
        let out_dir = tempdir()
            .expect("Could not create temporary directory for test")
            .keep();
        let out_dir_arg = out_dir
            .to_str()
            .expect("Could not convert temp dir name to string");

        let dir = super::get_output_dir(&url, &Some(out_dir_arg.to_string()), CompileType::Flow)
            .expect("Could not get output dir");

        assert_eq!(dir, out_dir);
        assert!(dir.exists());
    }
}
