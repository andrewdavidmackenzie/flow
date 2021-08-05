# `flowstdlib`

See also: [Code docs](http://andrewdavidmackenzie.github.io/flow/code/doc/flowstdlib/index.html)

`flowstdlib` is a standard library of functions anf flows for `flow` programs to use.

The directory containing this library must be part of `FLOW_LIB_PATH` or specified using an instance of the `-L` 
command line option to `flowc`, in order for the compiler to be able to find the library `flow` and `function` definitions.

Libraries like `flowstdlib` are built using `flowc` with the `-l` option.

It can be compiled and linked natively to a run-time, or each function can be compiled to WebAssembly and loaded
from file by `flowrlib` functions. The directory containing the compiled WebAssembly `.wasm` files must be reachable
by the run-time in order to load the functions at run-time.

The `flowc` compile process generates a `manifest.json` manifest of the libraries functions and where they 
implementations (.wasm files) can be found for loading by the runtime. 

It also generates a native `lib.rs` version of the manifest that can be used when linking the native compilations
of the implementations into a binary, such as `flowr`.

If you are using it as part of the larger `flow` workspace then you just need the `flow` project root directory
in your `FLOW_LIB_PATH` as described above (or added to the lib search part using the `-L <dir>` option).

If you download it using `cargo download` (or similar) and build separately, then you can get more information on how
to use it running `cargo run` (but it amounts to use of `FLOW_LIB_PATH` and `-L` as above).

If you install it using `cargo` then it will build the `flowstdlib` binary and add it to your path, and you can run it
directly using `> flowstdlib` and it will print the same usage hints as described above.