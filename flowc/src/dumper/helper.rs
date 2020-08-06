use std::fs::File;
use std::path::PathBuf;
use std::io;

pub fn create_output_file(output_path: &PathBuf, filename: &str, extension: &str) -> io::Result<File> {
    let mut output_file = PathBuf::from(filename);
    output_file.set_extension(extension);
    let mut output_file_path = output_path.clone();
    output_file_path.push(&output_file);
    File::create(&output_file_path)
}