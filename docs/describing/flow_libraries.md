## Flow Libraries
Libraries can provide functions (definition and implementation) and flows (definition) that can be re-used by other 
flows.

An example library is the `flowstdlib` library, but others can be created and shared by developers.

### Library structure
A flow library's structure is upto the developer to determine, starting with a `src` subdirectory, with optional 
sub-directories for modules, and sub-modules.

#### Native crate structure 
In order to support native linking of lib, it must be a valid rust crate and so a `Cargo.toml` file in the source
that references a `lib.rs` file, that in turn references `mod.rs` files in sub folder that reference the sources, so 
that it is all included into the crate when compiled. 

Example
```
[lib]
name = "flowstdlib"
path = "src/lib.rs"
```

#### Parallel WASM crate structure - WASM library build speed-up
Each function (see below) contains it's own `Cargo.toml` used to compile it to WASM. If left like this, then
each function will re-compile all of the source dependencies, even if many of the dependencies are shared across
all the functions, making the library compile to WASM very slow.

To speed up library builds, a solution ("hack") is used. A cargo workspace is defined in parallel with the Native 
crate mentioned above, with it's root workspace [Cargo.toml](../../flowstdlib/src/Cargo.toml) in the {lib_name}/src/
folder. This workspace includes as members references to all the `Cargo.toml` files of the functions (see below).
Thus when any of them are compiled they share a single target directory and the common dependencies are only
compiled once

#### Including a flow
Flow definition files may reside at any level. Example, the [sequence](../../flowstdlib/src/math/sequence.toml) flow definition 
in the `math` module of the `flowstdlib` library.

Alongside the flow definition a documentation Markdown file (with `.md` extension) can be included. It should be
referenced in the flow definition file using the `docs` field (e.g. `docs = "sequence.md"`).

#### Including a function
Each function should have a subdirectory named after function (`{function_name}`), which should include:
- `Cargo.toml` - build file for rust implementations
- `{function_name}.toml` - function definition file. It should include these fields
  - `type = "rust"` - type is obligatory and "rust" is the only type currently implemented
  - `function = "{function_name}"` - obligatory 
  - `source = "{function_name}.rs"` - obligatory and file must exist. 
  - `docs = "{function_name}.md"` - optional documentation file that if referenced must exist
- `{function_name}.md` - if references in function definition file then it will be used (copied to output)
- `{function_name}.rs` - referenced from function definition file. Must be valid rust and implement required traits

### Compiling a library
Flow libraries can be compiled using the `flowc` flow compiler and its `-l, --lib` option. This will compiler
and/or copy all required files from the library source directory into a library directory structure (where
can be specified with the `-o, --output <OUTPUT_DIR>` option). This directory is a self-contained, portable
library. It can be packaged, moved, unpackaged and used elsewhere, providing it can be found by the compiler
and runtime (using `FLOW_LIB_PATH` env var and `-L, --libdir <LIB_DIR|BASE_URL>` options) when needed.

The output directory structure will have the same structure as the library source (subdirs for modules) and will
include:
- `manifest.json` - Generated Library manifest, in the root of the directory structure
- `*.md` - Markdown source files copied into output directory corresponding to source directory
- `*.toml` - Flow and Function definition files copied into output directory corresponding to source directory
- `*.wasm` - Function WASM implementation compiled from supplied function source and copied into output 
  directory corresponding to source directory
- `*.dot` - 'dot' (graphvis) format graph descriptions of any flows in the library source
- `*.dot.svg` - flow graphs rendered into SVG files from the corresponding 'dot' files. These can be referenced in 
  doc files

### Lib References
References to flows or functions are described in more detail in the [process references](process_references.md)
section. Here we will focus on specifying the source for a process (flow or function) from a library using the "lib://"
Url format.

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