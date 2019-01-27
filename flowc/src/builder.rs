use std::path::PathBuf;

use flowrlib::manifest::Manifest;

pub fn build_implementations(out_dir: &PathBuf, manifest: &Manifest) -> Result<String, String> {
    for process in &manifest.processes {
        let source = process.implementation_source();
        let parts: Vec<_> = source.split(":").collect();
        match parts[0] {
            "lib" | "http" | "https" => {/* The implementation is in a library, so nothing to build */ }
            _ => build_implementation(out_dir, source)
        }
    }

    Ok("jobs".to_string())
}

fn build_implementation(_out_dir: &PathBuf, _source_path: &str) {
    info!("Provided function at '{}' needs building", _source_path);


}