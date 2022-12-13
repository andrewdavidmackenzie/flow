use std::fs::File;
use std::path::Path;
use std::path::PathBuf;

/// Module to dump a flow, or functions to .dot files of directed graphs
pub mod flow_to_dot;
/// Module to output the graph of functions after compilation
pub mod functions_to_dot;

/// Create a file at the specified `output_path`, `filename` and `extension` that output will be dumped to
pub(crate) fn create_output_file(
    output_path: &Path,
    filename: &str,
    extension: &str,
) -> std::io::Result<File> {
    let mut output_file = PathBuf::from(filename);
    output_file.set_extension(extension);
    let mut output_file_path = output_path.to_path_buf();
    output_file_path.push(&output_file);
    File::create(&output_file_path)
}
