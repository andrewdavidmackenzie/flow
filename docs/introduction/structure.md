## Project Components and Structure

Here is a summary of the project components, their purpose and a link to their `README.md`:

* [flowcore](../../flowcore/README.md) - A set of core structs and traits used by `flowr` and `flowc` plus code 
  to fetch content from file/http and resolve library (lib://) references.
* [flow_impl_derive](../../flow_impl_derive/README.md) - A derive macro used to help functions be compiled natively 
  and to wasm
* [flowc](../../flowc/README.md) - The `flowc` flow compiler binary is a CLI built around `flowclib` that 
  takes a number of command line arguments and source files or URLs and compiles the flow or library referenced.
    * `flowclib` is the library for compiling flow program and library definitions from toml 
      files, producing generated output projects that can be run by `flowr`.
* [flowr](../../flowr/README.md) - The `flowr` flow runner binary that can be used to run and debug flows compiled 
  with a flow compiler such as `flowc`.
    * [flowrlib](../../flowr/src/lib/README.md) - The flow runner library that loads and executes compiled flows.
        * [flowruntime](../../flowr/src/lib/flowruntime/README.md) - A set of core functions provided by the flow runtime 
          for all flows to interact with the environment and perform IO.
* [flowstdlib](../../flowstdlib/README.md) - the flow "standard library" which contains a set of functions that can be 
  used by flows being defined by the user
* [samples](../../samples/README.md) - A set of sample flows