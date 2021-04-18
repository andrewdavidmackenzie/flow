# `flowstdlib`

See also: [Code docs](http://andrewdavidmackenzie.github.io/flow/code/doc/flowstdlib/index.html)

`flowstdlib` is a standard library of functions for `flow` programs to use.

The directory containing this library must be part of `FLOW_LIB_PATH` or specified using an instance of the `-L` 
command line option, in order for the compiler to be able to find the library `flow` and `function` definitions.

It is build by using `flowc` with the `-l` option.
                      
It can be compiled and linked natively to a run-time, or each function can be compiled to WebAssembly and loaded 
from file by the run-time. The directory containing the compiled WebAssembly `.wasm` files must be reachable 
by the run-time in order to load the functions at run-time.

If you are using it as part of the larger `flow` workspace then you just need the `flow` project root directory
in your `FLOW_LIB_PATH` as described above (or added to the lib search part using the `-L <dir>` option).

If you download it using `cargo download` (or similar) and build separately, then you can get more information on how
to use it running `cargo run` (but it amounts to use of `FLOW_LIB_PATH` and `-L` as above).

If you install it using `cargo` then it will build the `flowstdlib` binary and add it to your path, and you can run it
directly using `> flowstdlib` and it will print the same usage hints as described above.