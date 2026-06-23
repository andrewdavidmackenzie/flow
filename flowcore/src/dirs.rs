//! Platform-standard directory paths for flow data.
//!
//! Uses the `directories` crate to find the correct location per platform:
//! - Linux: `~/.local/share/flow/`
//! - macOS: `~/Library/Application Support/flow/`
//! - Windows: `C:\Users\{user}\AppData\Roaming\flow\data\`

use std::path::PathBuf;

use directories::ProjectDirs;

fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", "flow")
}

/// Return the base data directory for flow.
#[must_use]
pub fn data_dir() -> Option<PathBuf> {
    project_dirs().map(|dirs| dirs.data_dir().to_path_buf())
}

/// Return the default library directory (`{data_dir}/lib/`).
#[must_use]
pub fn lib_dir() -> Option<PathBuf> {
    data_dir().map(|d| d.join("lib"))
}

/// Return the default runner directory for a given runner name
/// (`{data_dir}/runner/{runner_name}/`).
#[must_use]
pub fn runner_dir(runner_name: &str) -> Option<PathBuf> {
    data_dir().map(|d| d.join("runner").join(runner_name))
}
