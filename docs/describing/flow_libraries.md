## Flow Libraries
Libraries can provide Functions and Flows in order to be re-used by other flows.
A library can provide a definition of a flow that can be re-used elsewhere, or a function.
In the case of the function the library must provide both the definition and it's implementation.

An example library is the `flowstdlib` library, but others can be created and shared by developers.

References to flows or functions are described in more detail in the [process references](process_references.md)
section. Here we will focus on specifying the source for a process (flow or function) from a library using the "lib://"
Url format.

### Lib References
The process reference to refer to a library provided flow or function is of the form:
`lib://lib_name/path_to_flow_or_function`

Breaking that down:
- "lib://" Url scheme identifies this reference as a reference to a library provided flow or function
- "lib_name" (the hostname of the Url) is the name of the library
- "path_to_flow_or_function" (the path of the Url) is the location *withing* the library where the flow or function 
  resides.

By not specifying a location (a file with `file://` or web resource with `http://` or `https://`) allows the system
to load the actual library with it's definitions and implementation from different places in different `flow` 
installations thus flows that use library functions are portable, providing the library is present and can be found 
wherever it is being run.

The `flowrlib` runtime library accepts a "search path" where it should look for the library (using the library's
name "lib_name" from the Url)

Different flow runners (e.g. `flowr` or `flowrex` are included examples here but others can be written) can provide
different ways to provide entries in the search path. Below we describe how `flowr` and `flowrex` do this.

### Configuring the Library Search Path
The library search path is initialized from the contents of the `$FLOW_LIB_PATH` environment variable
(if it is defined).
This path maybe augmented by supplying additional directories or URLs to search using one
or more instances of the `-L` command line option.

### Finding the references lib process
The algorithm used to find files via process references is described in more detail in the 
[process references](process_references.md) section. An example of how a library function is found is shown below.

A process reference exists in a flow with `source = "flowstdlib://math/add"`
  * Library name = `flowstdlib`
  * Function path within the library = `math/add`

All the directories in the search path are searched for a top-level sub-directory that matches the library name.

If a directory matching the library name is found, the path to the process within the library is used to try and
find the process definition file.

For example, if `FLOW_LIB_PATH` environment variable is defined thus:
* `export FLOW_LIB_PATH=/Users/me/workspace/flow`

And the flow references a process thus:
```toml
[[process]]
source = "flowstdlib://math/add"
```

Then the directory `/Users/me/workspace/flow/flowstdlib` is looked for.

If that directory is found, then the process path within the library `stdio/stdin` is used to create the full path
to the process definition file is `/Users/me/workspace/flow/flowstdlib/math/add`.

(refer to the full algorithm in [process references](process_references.md))

If the file `/Users/me/workspace/flow/flowstdlib/math/add.toml` exists then it is parsed and made available to the flow
for use in connections.