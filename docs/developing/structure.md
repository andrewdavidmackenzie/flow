## Project Structure

Here is a summary of the project sub-folders, their purpose and a link to their `README.md`:

* [flow_impl](../../flow_impl/README.md) - Definition of trait that functions must implement
* [flow_impl_derive](../../flow_impl_derive/README.md) - A derive macro used to help functions be compiled natively and to wasm
* [flowc](../../flowc/README.md) - The `flowc` flow compiler binary and `flowclib` library for compiling flow program and library definitions
* [flowr](../../flowr/README.md) - The `flowr` flow runner binary for executing flows
* [flowrlib](../../flowr/src/lib/flowrlib.md) - The flow runner library that loads and executes flows
* [flowruntime](../../flowr/src/lib/flowruntime/flowruntime.md) - A set of core functions provided by any flow runtime for all flows to use 
* [flowrstructs](../../flowrstructs/README.md) - A set of core structs used by `flowr` and `flowc`
* [flowstdlib](../../flowstdlib/README.md) - A library of basic functions and flows to be used by processes
* [provider](../../provider/README.md) - Library used to fetch content from file/http and refer to libraries
* [samples](../../samples/README.md) - A set of sample flows 
