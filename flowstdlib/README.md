# `flowstdlib`

See also: [Code docs](http://andrewdavidmackenzie.github.io/flow/code/doc/flowstdlib/index.html)

`flowstdlib` is a standard library of functions for `flow` programs to use.

The directory containing this library must be part of `FLOW_LIB_PATH` in order for the compiler to be able to 
find the library `flow` and `function` definitions.

It is build by using `flowc` with the `-l` option.
                      
It can be compiled and linked natively to a run-time, or each function can be compiled to WebAssembly and loaded 
from file by the run-time. The directory containing the compiled WebAssembly `.wasm` files must be reachable 
by the run-time in order to load the functions at run-time.