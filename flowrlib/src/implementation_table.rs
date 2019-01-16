use super::implementation::Implementation;
use std::collections::HashMap;

// Key = ImplementationSource, Value = ImplementationLocator
pub type ImplementationTable<'a>  = HashMap<String, &'a dyn Implementation>;

/*
    Implementations can be of two types - either a native and statically bound function referenced
    via a function reference, or WASM bytecode file that is interpreted at run-time that is
    referenced via a PathBuf pointing to the .wasm file
*/
#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum ImplementationLocator<'a> {
    #[serde(skip_deserializing, skip_serializing)]
    Native(&'a dyn Implementation),
    Wasm(&'a str),
}

/*
    Provided by libraries to help load and/or find implementations of processes
*/
pub type ImplementationLocatorTable<'a> = HashMap<&'a str, ImplementationLocator<'a>>;


