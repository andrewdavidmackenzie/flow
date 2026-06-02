## Provided Functions
As described previously, flows can use `provided functions` provided by the flow runner app (e.g. `flowrcli`)
and by flow libraries.

However, a flow can also provide its own functions (a definition, for the compiler, and an implementation, 
for the runtime).

The [process references](process_references.md) section describes the algorithm for finding the function's files 
(definition and implementation) using relative paths within a flow file hierarchy.

Using relative paths means that flows are "encapsulated" and portable (by location) as they can be moved
between directories, files systems and systems/nodes and the relative locations of the provided functions allow 
them to still be found and the flow compiled and ran.

### Examples of Provided Functions
The `flowr` crate has several examples that provide functions as part of the flow:
* [Mandlebrot](../../flowr/examples/mandlebrot/DESCRIPTION.md) in the folder `flowr/examples/mandlebrot` - provides
  two functions:
  * `pixel_to_point` to do conversions from pixels to points in 2D imaginary
    coordinates space
  * `escapes` to calculate the value of a point in the mandlebrot set

### What a provided function has to provide
In order to provide a function as part of a flow the developer must provide:

#### Function definition file
Definition of the function in a TOML file.   
Example [escapes.toml](../../flowr/examples/mandlebrot/escapes/escapes.toml)  
The same as any other function definition it must define:
   * `function` - field to show this is a function definition file and provide the function's name 
   * `source` - the name of the implementation file (relative path to this file)
   * `type` - to define what type of implementation is provided (`"rust"` is the only supported value at this time)
   * `input`- the function's inputs - as described in [IOs](ios.md)
   * `output`- the function's outputs - as described in [IOs](ios.md)
   * `docs` - Documentation markdown file (relative path)  
Example [escapes.md](../../flowr/examples/mandlebrot/escapes/escapes.md)

#### Implementation
Code that implements the function of the type specified by `type` in the file specified by `source`.  
Example: [escapes.rs](../../flowr/examples/mandlebrot/escapes/escapes.rs)

This may optionally include tests, that will be compiled and run natively.

### Writing function implementations

Function implementations use the `#[flow_function]` macro from `flowmacro`. The macro
generates boilerplate code for input extraction, type checking, and WASM interop.

#### Typed input parameters

Instead of manually extracting inputs from a `&[Value]` slice, declare typed parameters
that match the input names in the function's TOML definition:

```rust
use serde_json::{json, Value};
use flowcore::errors::Result;
use flowcore::{RunAgain, RUN_AGAIN};
use flowmacro::flow_function;

#[flow_function]
fn inner_add(i1: &Value, i2: &Value) -> Result<(Option<Value>, RunAgain)> {
    // i1 and i2 are extracted and type-checked by the macro
    // No manual inputs.first().ok_or(...)? needed
    Ok((Some(json!(1)), RUN_AGAIN))
}
```

Supported parameter types:

| Rust type | Flow type | What the macro generates |
|-----------|-----------|------------------------|
| `&Value` | generic | `inputs.get(i).ok_or(...)` |
| `Value` | generic | `inputs.get(i).ok_or(...)?.clone()` |
| `&Number` | number | `inputs.get(i).ok_or(...)?.as_number().ok_or(...)` |
| `f64` | number | `inputs.get(i).ok_or(...)?.as_f64().ok_or(...)` |
| `i64` | number | `inputs.get(i).ok_or(...)?.as_i64().ok_or(...)` |
| `bool` | boolean | `inputs.get(i).ok_or(...)?.as_bool().ok_or(...)` |
| `&str` | string | `inputs.get(i).ok_or(...)?.as_str().ok_or(...)` |

The parameter names must match the input names in the TOML definition (hyphens
are normalized to underscores). The macro validates this at compile time.

#### Named outputs with `flow_output!`

For functions with multiple named outputs, use the `flow_output!` macro instead
of manually building a `serde_json::Map`:

```rust
use flowcore::flow_output;
use serde_json::json;

// Instead of:
//   let mut map = serde_json::Map::new();
//   map.insert("result".into(), json!(33));
//   map.insert("remainder".into(), json!(1));
//   Ok((Some(Value::Object(map)), RUN_AGAIN))

// Use:
flow_output!(
    "result" => json!(33),
    "remainder" => json!(1)
)
```

The macro builds the output map and returns `Ok((Some(map), RUN_AGAIN))`.

For functions with a single unnamed output, return the value directly:

```rust
Ok((Some(json!(result)), RUN_AGAIN))
```

#### Build file
In the case of the `rust` type (the only type implemented!), a `Cargo.toml` file that is used to compile
the function's implementation to WASM as a stand-alone project.

### How are provided function implementations loaded and ran
If the flow running app (using the `flowrlib` library) is statically linked, how can it load and then run the
provided implementation?

This is done by compiling the provided implementation to WebAssembly, using the provided build file. The .wasm
byte code file is generated when the flow is compiled and then loaded when the flow is loaded by `flowrlib`.