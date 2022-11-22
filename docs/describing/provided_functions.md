## Provided Functions
As described previously, flows can use `provided functions` provided by the flow runner app (e.g. `flowr`)
and by flow libraries.

However, a flow can also provide its own functions (a definition, for the compiler, and an implementation, 
for the runtime).

The [process references](process_references.md) section describes the algorithm for finding the function's files 
(definition and implementation) using relative paths within a flow file hierarchy.

Using relative paths means that flows are "encapsulated" and portable (by location) as they can be moved
between directories, files systems and systems/nodes and the relative locations of the provided functions allow 
them to still be found and the flow compiled and ran.

### Examples
The `flowsamples` crates has two samples that provide functions as part of the flow:
* [Reverse Echo](../../flowsamples/reverse-echo/DESCRIPTION.md) in the folder `flowsamples/reverse-echo` - a
  simple sample that provides a function to reverse a string
* [Mandlebrot](../../flowsamples/mandlebrot/DESCRIPTION.md) in the folder `flowsamples/mandlebrot` - provides
  two functions:
  * `pixel_to_point` to do conversions from pixels to points in 2D imaginary
    coordinates space
  * `escapes` to calculate the value of a point in the mandlebrot set

### What a provided function has to provide
In order to provide a function as part of a flow the developer must provide:

#### Function definition file
Definition of the function in a TOML file.   
Example [escapes.toml](../../flowsamples/mandlebrot/escapes/escapes.toml)  
The same as any other function definition it must define:
   * `function` - field to show this is a function definition file and provide the function's name 
   * `source` - the name of the implementation file (relative path to this file)
   * `type` - to define what type of implementation is provided (`"rust"` is the only supported value at this time)
   * `input`- the function's inputs - as described in [IOs](ios.md)
   * `output`- the function's outputs - as described in [IOs](ios.md)
   * `docs` - Documentation markdown file (relative path)  
Example [escapes.md](../../flowsamples/mandlebrot/escapes/escapes.md)

#### Implementation
Code that implements the function of the type specified by `type` in the file specified by `source`.  
Example: [escapes.rs](../../flowsamples/mandlebrot/escapes/escapes.rs)

This may optionally include tests, that will be compiled and run natively.

#### Build file
In the case of the `rust` type (the only type implemented!), a `Cargo.toml` file that is used to compile 
the function's implementation to WASM as a stand-along project. 

### How are provided function implementations loaded and ran
If the flow running app (using the `flowrlib`library`) is statically linked, how can it load and then run the
provided implementation?

This is done by compiling the provided implementation to WebAssembly, using the provided build file. The .wasm
byte code file is generated when the flow is compiled and then loaded when the flow is loaded by `flowrlib`