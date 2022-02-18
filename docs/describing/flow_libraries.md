## Flow Libraries
Libraries can provide Functions and Flows and be re-used by other flows.

An example is the `flowstdlib` library, but others can be created and shared by developers.

References to flows or functions specify a source, which refers to a file on the local file system 
or located at a URL. 
A Library search path is used to find the library sources

The library search path is initialized from the contents of the `$FLOW_LIB_PATH` environment variable 
(if it is defined) and maybe augmented by supplying additional directories or URLs to search using one 
or more instances of the `-L` command line option.

TODO

Format and examples of how to reference library functions from flows