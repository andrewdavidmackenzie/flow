## Provided Functions
As described previously, flows can use functions provided by the `context` (flow runner app) and by flow libraries.

However, a flow can also provide its own functions (a definition, for the compiler, and an implementation, for the
runtime).

The [process references](process_references.md) section describes the algorithm for finding the function's files 
(definition
and implementation) using relative paths within a flow file hierarchy.

Using relative paths means that flows are "encapsulated" and portable (by location) as they can be moved around
between directories, files systems and systems/nodes and the relative locations of the provided functions allow 
them to still be found and the flow compiled and ran.

Examples can be found:
* `Reverse Echo` in the folder `flowsamples/reverse-echo` - a simple sample that provides an implementation of a 
  function to reverse a string
* `Mandlebrot` in the folder `flowsamples/mandlebrot` - a more complex sample that calculates a mandlebrot set and 
  creates an image of it, including provided implementation for two functions used in the process `escapes` and
`pixel_to_point`

### What a provided function has to provide
In order to provide a function as part of a flow the developer must provide:
* Function definition in a TOML file. Example [escapes.toml](../../flowsamples/mandlebrot/escapes/escapes.toml)
As for other function definitions it must define
   * `function` - field to show this is a function definition file and provide the function's name 
   * `source` - the name of the implementation file 
   * `type` - to define what type of implementation is provided (`"rust"` is the only supported value at this time)
   * `input`- the function's inputs - as described in [IOs](ios.md)
   * `output`- the function's outputs - as described in [IOs](ios.md)
* Documentation markdown file, if one is referenced from the definition file. Example
[escapes.md](../../flowsamples/mandlebrot/escapes/escapes.md)
* Implementation - Code that implements the function of the type specified by `type` in the definition file. 
Example: [escapes.rs](../../flowsamples/mandlebrot/escapes/escapes.rs)
* Build file - in the case of the `rust` type, a Cargo.toml file that is used to compile the function's 
implementation as a stand-along project. This may optionally include tests.

### How are provided function implementations loaded and ran
If the flow running app (using the `flowrlib`library`) is statically linked, how can it load and then run the
provided implementation?

This is done by compiling the provided implementation to WebAssembly, using the provided build file. The .wasm
byte code file is generated when the flow is compiled and then loaded when the flow is loaded by `flowrlib`