use std::path::PathBuf;
use std::io::{Error, ErrorKind};
use std::io::Result;
use std::fs;
use model::runnable::Runnable;

pub fn copy(src_dir: &PathBuf, runnables: &Vec<Box<Runnable>>) -> Result<()> {
    // Find all the functions that are not loaded from libraries
    for runnable in runnables {
        if let Some(source_url) = runnable.source_url() {
            let mut source = source_url.to_file_path()
                .map_err(|_e| Error::new(ErrorKind::InvalidData, "Could not convert to file path"))?;
            source.set_extension("rs");
            let mut dest = src_dir.clone();
            dest.push(source.file_name().unwrap());
            fs::copy(source, dest)?;
        }
    }

    Ok(())
}