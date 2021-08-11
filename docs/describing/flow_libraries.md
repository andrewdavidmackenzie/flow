## Flow Libraries
Functions and Flows can be defined as libraries and shared re-used by other flows.

The `flowrlib` lib and hence the `flowr` binary include the `flowstdlib` library, natively linked for speed as well
as loading it's WASM implementations.

References to flows or functions specify a source, which is a file on the local file system or located at a URL. 
A Library search path is used to find the source in libraries.

The library search path is initialized from the contents of the `$FLOW_LIB_PATH` environment variable (if it is defined) 
and maybe augmented by supplying additional directories or URLs to search using one or more instances of 
the `-L` command line option.

TODO

Format and examples of how to reference library functions from flows