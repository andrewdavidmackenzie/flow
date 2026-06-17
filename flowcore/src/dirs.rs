//! Platform-standard directory paths for flow data.
//!
//! Uses the `directories` crate to find the correct location per platform:
//! - Linux: `~/.local/share/flow/`
//! - macOS: `~/Library/Application Support/flow/`
//! - Windows: `C:\Users\{user}\AppData\Roaming\flow\`
//!
//! Falls back to the legacy `~/.flow/` location if the standard path doesn't
//! exist but the legacy one does, for backward compatibility.

use std::path::PathBuf;

use directories::ProjectDirs;

fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", "flow")
}

fn legacy_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(|h| PathBuf::from(h).join(".flow"))
}

/// Return the base data directory for flow.
/// Prefers the standard platform directory, falls back to `~/.flow/` if it exists.
#[must_use]
pub fn data_dir() -> Option<PathBuf> {
    let standard = project_dirs().map(|dirs| dirs.data_dir().to_path_buf());

    // If the standard path exists, use it
    if let Some(ref dir) = standard {
        if dir.exists() {
            return standard;
        }
    }

    // If the legacy path exists, use it for backward compatibility
    if let Some(legacy) = legacy_dir() {
        if legacy.exists() {
            return Some(legacy);
        }
    }

    // Neither exists — return the standard path (it will be created on first use)
    standard
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
