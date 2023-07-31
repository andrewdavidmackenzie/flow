## Running Flows

In order to run a flow, it must first be compiled. Then a "flow runner" (such as `flowrcli`) can be used to run the
compiled flow manifest.

For convenience, `flowc`, the flow compiler, compiles the flow, then uses `flowrcli` to run it for you (unless you 
specify otherwise). So that is the easiest way to run a flow and is used below.

If you have run `make` or `make install_flow` then you will have `flowc` and `flowrcli` installed on your system.
Be sure they are in your $PATH so that can be invoked directly.

Then you can run flows easily using `flowc`. 

If you do not wish to install `flowc` then you can run it using `cargo` from the root of the project directory by
substituting `cargo run -p flowc --` for `flowc`in the examples below.

User's terminal Current Working Directory should be the root directory of the `flow` project

### Compiling `flowstdlib` if you have used `cargo install` to install `flowstdlib`
If you have not compiled the project from source using `make`, then it's probable that `flowstdlib` has not
been compiled to WASM. However, there should be a `flowstdlib` binary on your system. This should be run, passing it
the path to the flowstdlib source folder (the root, not `src` inside it) in order to compile it.

This will take considerable time, and leave the compiled WASM files in `$HOME/.flow/flib/flowstdlib`

### Finding Libraries
Environment variable `$FLOW_LIB_PATH` is set to a directory where the `flowstdlib` is compiled by default 
`$HOME/.flow/lib`, in order for `flowc` and `flowrcli` to be able to find library functions used.

If this environment variable is not set then compiling will fail:

```
❯ unset FLOW_LIB_PATH
❯ flowc -C flowr/src/bin/flowrcli/context flowr/examples/fibonacci
error: Could not resolve the url: 'lib://flowstdlib/math/add'
caused by: Could not resolve library Url 'lib://flowstdlib/math/add' using Search Path 'FLOW_LIB_PATH': Directories: {}, URLs: {}
```

Directories to add to the library search path to help find libraries used can be passed to `flowc` via one or more
instances of the `-L, --libdir <LIB_DIR|BASE_URL>` Option (see below for an example).

### Full List of `flowc` Options
See the next section [flowc](flowc.md) for a description of the command line arguments it accepts.

### Example Invocations
- `flowc -C flowr/src/bin/flowrcli/context flowr/examples/fibonacci`

  uses the `context_functions` provided by `flowrcli` and run the flow whose root flow is defined in `./flowr/examples/fibonacci/root.toml`. 
  Do not pass in any arguments to the flow. 
  - You should get a fibonacci series output to the terminal, 
- `echo "Hello" | flowc -C flowr/src/bin/flowrcli/context flowr/examples/reverse-echo` - This example reads from STDIN so we echo in 
  some text.
  - You may see some output like:
  
    `Testing /Users/andrew/workspace/flow/flowr/examples/reverse-echo/reverse/Cargo.toml WASM Project
     Compiling /Users/andrew/workspace/flow/flowr/examples/reverse-echo/reverse/Cargo.toml WASM project`

    the first time this example is run as the `provided function` is tested and compiled to WASM, followed by

    `olleH`

    which is the input string "Hello" reversed.
- `unset FLOW_LIB_PATH;flowc -C flowr/src/bin/flowrcli/context -L target flowr/examples/fibonacci` - first ensures that the $FLOW_LIB_PATH
environment variable is not set and is not being used to locate libraries, and in order to help `flowc` and `flowrcli` 
find the `flowstdlib` library used by the example (previously compiled into `target` directory) it specified that as a
directory for the library search path using the `-L, --libdir <LIB_DIR|BASE_URL>` Option
  - You should get a fibonacci series output to the terminal, 
- `flowc -C flowr/src/bin/flowrcli/context flowr/examples/sequence 10` - as previous examples except that after the `source_url` a 
`flow_argument` of "10" is passed in
  - A short sequence of numbers (2, 5, 8) and a string will be printed. The "10" represents the maximum of the sequence.

### Running a flow from the web
As stated, the `source_url` can be a Url to a web resource, or a flow definition hosted on a web server.

### Example running a flow from the web
We can use a flow that is part of the `flow` project, where the flow definition is hosted on the web by GitHub:

`flowc -C flowr/src/bin/flowrcli/context "https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/flowcore/tests/test-flows/hello-world/root.toml"`


That will pull the flow definition content from the web, compile it and run it, producing the expected output:


`Hello World!`