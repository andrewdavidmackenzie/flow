use std::collections::HashMap;
use std::fs::File;
use std::io::{Error, ErrorKind, Read};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use flowrlib::implementation::Implementation;
use flowrlib::process::Process;
use serde_json;

// Key = ImplementationSource, Value = ImplementationLocator
pub type ImplementationTable<'a>  = HashMap<String, &'a Implementation>;

pub fn get_processes<'a>(path: &PathBuf) -> Result<Vec<Process<'a>>, Error> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    serde_json::from_str(&contents)
        .map_err(|e| Error::new(ErrorKind::Other,
        format!("Could not deserialize json file '{}'\nError = '{}'",
                path.display(), e)))
}

pub fn load_flow<'a>(path: &PathBuf) -> Result<Vec<Arc<Mutex<Process<'a>>>>, Error> {
    let processes = get_processes(path)?;
    let mut runnables = Vec::<Arc<Mutex<Process>>>::new();

    for process in processes {
        runnables.push(Arc::new(Mutex::new(process)));
    }

    Ok(runnables)
}