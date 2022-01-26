use std::path::Path;
use std::sync::{Arc, Mutex};

use log::{debug, trace};
use url::Url;

use flowcore::errors::*;
use flowcore::lib_provider::Provider;

use crate::client_server::ServerConnection;
use crate::runtime_messages::{ClientMessage, FileMetaData, ServerMessage};

/// `client_provider` is a special content provider that makes requests to the client to fetch files
pub struct ClientProvider {
    runtime_server_connection: Arc<Mutex<ServerConnection>>,
}

impl Provider for ClientProvider {
    fn resolve_url(
        &self,
        url: &Url,
        default_filename: &str,
        extensions: &[&str],
    ) -> Result<(Url, Option<String>)> {
        let path = url
            .to_file_path()
            .map_err(|_| format!("Could not convert '{}' to a file path", url))?;

        let md_result = self
            .metadata(&path)
            .chain_err(|| "Error getting file metadata for path");

        match md_result {
            Ok(md) => {
                if md.is_dir {
                    trace!(
                        "'{}' is a directory, so attempting to find default file named '{}' in it",
                        path.display(),
                        default_filename
                    );
                    if let Ok(file_found_url) = self.find_file(&path, default_filename, extensions)
                    {
                        return Ok((file_found_url, None));
                    }

                    trace!(
                        "'{}' is a directory, so attempting to find file with same name inside it",
                        path.display()
                    );
                    if let Some(dir_os_name) = path.file_name() {
                        let dir_name = dir_os_name.to_string_lossy();
                        if let Ok(file_found_url) = self.find_file(&path, &dir_name, extensions) {
                            return Ok((file_found_url, None));
                        }
                    }

                    bail!("No default or same named file found in directory")
                } else if md.is_file {
                    Ok((url.clone(), None))
                } else {
                    let file_found_url = self.file_by_extensions(&path, extensions)?;
                    Ok((file_found_url, None))
                }
            }
            _ => {
                // doesn't exist
                let file_found_url = self.file_by_extensions(&path, extensions)?;
                Ok((file_found_url, None))
            }
        }
    }

    fn get_contents(&self, url: &Url) -> Result<Vec<u8>> {
        match self.runtime_server_connection.lock() {
            Ok(mut guard) => {
                let path = url
                    .to_file_path()
                    .map_err(|_| format!("Could not convert '{}' to a file path", url))?;
                match guard.send_and_receive_response(ServerMessage::Read(path)) {
                    Ok(ClientMessage::FileContents(_, contents)) => Ok(contents),
                    _ => bail!("Error while reading file on client"),
                }
            }
            _ => bail!("Server could not lock context"),
        }
    }
}

impl ClientProvider {
    /// Create a new client provider, using the provided Server Connection to the client
    pub fn new(
        runtime_server_connection: Arc<Mutex<ServerConnection>>,
    ) -> Self {
        ClientProvider {
            runtime_server_connection,
        }
    }

    fn metadata(&self, path: &Path) -> Result<FileMetaData> {
        match self.runtime_server_connection.lock() {
            Ok(mut guard) => {
                match guard
                    .send_and_receive_response(ServerMessage::GetFileMetaData(path.to_path_buf()))
                {
                    Ok(ClientMessage::FileMetaDate(_, md)) => Ok(md),
                    _ => bail!("Error while retrieving metadata of file  on client"),
                }
            }
            _ => bail!("Server could not lock context"),
        }
    }

    /// Passed a path to a directory, it searches for a file in the directory called 'default_filename'
    /// If found, it opens the file and returns its contents as a String in the result
    pub fn find_file(
        &self,
        dir: &Path,
        default_filename: &str,
        extensions: &[&str],
    ) -> Result<Url> {
        let mut file = dir.to_path_buf();
        file.push(default_filename);

        self.file_by_extensions(&file, extensions)
    }

    /// Given a path to a filename, try to find an existing file with any of the allowed extensions
    pub fn file_by_extensions(&self, file: &Path, extensions: &[&str]) -> Result<Url> {
        let mut file_with_extension = file.to_path_buf();

        // for that file path, try with all the allowed file extensions
        for extension in extensions {
            file_with_extension.set_extension(extension);
            debug!("Looking for file '{}'", file_with_extension.display());
            if let Ok(md) = self.metadata(&file_with_extension) {
                if md.is_file {
                    let file_path_as_url =
                        Url::from_file_path(&file_with_extension).map_err(|_| {
                            format!(
                                "Could not create url from file path '{}'",
                                file_with_extension.display()
                            )
                        })?;

                    return Ok(file_path_as_url);
                }
            }
        }

        bail!(
            "No file found at path '{}' with any of these extensions '{:?}'",
            file.display(),
            extensions
        )
    }
}
