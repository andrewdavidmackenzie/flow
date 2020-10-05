# `flowstdlib`

See also: [Code docs](http://andrewdavidmackenzie.github.io/flow/code/doc/flowstdlib/index.html)

A library of commonly used flows and functions that can be used by other `flows`.

The directory containing this library must be part of `FLOW_LIB_PATH` in order for the compiler to be able to 
find the library `flow` and `function` definitions.

The chosen run-time to execute your flow must either have it statically linked (a feature) or have it compiled 
(using `flowc` with `-l` option) and the directory containing the compiled WebAssembly `.wasm` files reachable
in order to load the functions at run-time.