use flowrlib::errors::*;
use flowrlib::provider::Provider;

//use std::io;
//use std::path::PathBuf;

pub struct FileProvider{
    content: String
}

impl FileProvider {
    pub fn new(content: String) -> Self {
        FileProvider{
            content
        }
    }
}

impl Provider for FileProvider {
    fn resolve_url(&self, url_str: &str, _default_filename: &str, _extensions: &[&str]) -> Result<(String, Option<String>)> {
/*
        match metadata(&path) {
            Ok(md) => {
                if md.is_dir() {
                    info!("'{}' is a directory, so attempting to find default filename in it",
                          path.display());
                    let file = FileProvider::find_default_file(&mut path, default_filename).
                        chain_err(|| format!("Could not find default file name '{}'", default_filename))?;
                    let resolved_url = Url::from_file_path(&file)
                        .chain_err(|_| format!("Could not create url from file path '{}'",
                                             file.to_str().unwrap()))?;
                    Ok((resolved_url.into_string(), None))
                } else {
                    Ok(c)
                }
            }
            Err(e) => {
                Err(format!("Error getting file metadata for path: '{}', {}", path.display(), e))
            }
        }
        */
        Ok((url_str.into(), None))
    }

    fn get_contents(&self, _url_str: &str) -> Result<Vec<u8>> {
//        let file = File::new(url_str);
//        let reader = FileReaderSync::new()?;
//        let result_base64 = reader.read_as_text(file);

        Ok(self.content.clone().into_bytes())

        /*
        let file_reader = FileReader::new();
        file_reader.onload = (function(reader)
        {
            return function()
            {
                var contents = reader.result;
                var lines = contents.split('\n');
                //////
                document.getElementById('container').innerHTML=contents;
            }
        })(reader);

        file_reader.readAsText(f);
        */

        /*
        let url = Url::parse(url_str)
            .chain_err(|_| format!("Could not convert '{}' to Url", url_str))?;
        let file_path = url.to_file_path().unwrap();
        let mut f = File::open(&file_path)
            .chain_err(|e| format!("Could not open file '{:?}' ({}", file_path, e))?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)
            .chain_err(|e| format!("Could not read content from '{:?}' ({}", file_path, e))?;
        Ok(buffer)
        */
    }
}

impl FileProvider {
    /*
        Passed a path to a directory, it searches for a file in the directory called 'default_filename'
        If found, it opens the file and returns its contents as a String in the result
    */

//    fn find_default_file(path: &mut PathBuf, _default_filename: &str) -> io::Result<PathBuf> {
        /*
        // TODO pending more complex patterns based on implemented loaders
        // Or iterate through the matches until a loader is found which understands that file extension
        path.push(default_filename);
        let pattern = path.to_str().unwrap();
        info!("Looking for files matching: '{}'", pattern);

        // Try to glob for the default file using a pattern
        for entry in glob(pattern).expect("Failed to read glob pattern") {
            // return first file found that matches the pattern, or error if none match
            match entry {
                Ok(context_file) => return Ok(context_file),
                Err(_) => return Err(io::Error::new(ErrorKind::NotFound,
                                                    format!("No context file found matching '{}'",
                                                            path.display())))
            }
        }

        // No matches
        Err(io::Error::new(ErrorKind::NotFound,
                           format!("No default context file found. Tried '{}'", path.display())))
                           */

//        Ok(path.clone())
//    }
}
