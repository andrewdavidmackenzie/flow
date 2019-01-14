

/*
    Implementations can be of two types - either a native and statically bound function referenced
    via a function reference, or WASM bytecode file that is interpreted at run-time that is
    referenced via a PathBuf pointing to the .wasm file
    // TODO probably will change the wasm one to a function in memory wrapping the loaded was.
    // maybe even be able to wrap it in Implementation as that's just a Trait
*/
pub enum ImplementationLocator<'a> {
    Native(&'a Implementation),
    Wasm(PathBuf)
}

/*
    Provided by libraries to help load and/or find implementations of processes
*/
pub type ImplementationLocatorTable<'a>  = HashMap<String, ImplementationLocator<'a>>;
