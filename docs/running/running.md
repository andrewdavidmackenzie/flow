## Running Flows

In order to run a flow, it must first be compiled. Then a "flow runner" (such as `flowr`can be used to run the compiled
flow manifest).

For convenience, `flowc`, the flow compiler, compiles the flow, then uses `flowr` to run it for you (unless you 
specify otherwise). So that is the easiest way to run a flow and is used below.

If you have run `make` or `make install_flow` then you will have `flowc` and `flowr` installed on your system.
Be sure they are in your $PATH so that can be invoked directly.

Then you can run flows easily using `flowc`. 

If you do not wish to install `flowc` then you can run it using `cargo` from the root of the project directory by
substituting `cargo run -p flowc --` for `flowc`in the examples below.

User's terminal Current Working Directory should be the root directory of the `flow` project

### Finding Libraries
Environment variable `$FLOW_LIB_PATH` is set to the target directory where the `flowstdlib` is compiled by default 
`${flow_root_dir}/target`, in order for `flowc` and `flowr` to be able to find library functions used.

If this environment variable is not set then compiling will fail:

```
❯ unset FLOW_LIB_PATH
❯ flowc -C flowr/src/cli flowsamples/fibonacci
error: Could not resolve the url: 'lib://flowstdlib/math/add'
caused by: Could not resolve library Url 'lib://flowstdlib/math/add' using Search Path 'FLOW_LIB_PATH': Directories: {}, URLs: {}
```

Directories to add to the library search path to help find libraries used can be passed to `flowc` via one or more
instances of the `-L, --libdir <LIB_DIR|BASE_URL>` Option (see below for an example).

### Full List of `flowc` Options
See the next section [flowc](flowc.md) for a description of the command line arguments it accepts.

### Example Invocations
- `flowc -C flowr/src/cli flowsamples/fibonacci`

  uses the `context_functions` provided by `flowr` and run the flow whose root flow is defined in `./flowsamples/fibonacci/root.toml`. 
  Do not pass in any arguments to the flow. 
  - You should get a fibonacci series output to the terminal, 
- `echo "Hello" | flowc -C flowr/src/cli flowsamples/reverse-echo` - This samples reads from STDIN so we echo in 
  some text.
  - You may see some output like:
  
    `Testing /Users/andrew/workspace/flow/flowsamples/reverse-echo/reverse/Cargo.toml WASM Project
     Compiling /Users/andrew/workspace/flow/flowsamples/reverse-echo/reverse/Cargo.toml WASM project`

    the first time this sample is run as the `provided function` is tested and compiled to WASM, followed by

    `olleH`

    which is the input string "Hello" reversed.
- `unset FLOW_LIB_PATH;flowc -C flowr/src/cli -L target flowsamples/fibonacci` - first ensures that the $FLOW_LIB_PATH
environment variable is not set and is not being used to locate libraries, and in order to help `flowc` and `flowr` 
find the `flowstdlib` library used by the sample (previously compiled into `target` directory) it specified that as a
directory for the library search path using the `-L, --libdir <LIB_DIR|BASE_URL>` Option
  - You should get a fibonacci series output to the terminal, 
- `flowc -C flowr/src/cli flowsamples/sequence 10` - as previous examples except that after the `source_url` a 
`flow_argument` of "10" is passed in
  - A short sequence of numbers (2, 5, 8) and a string will be printed. The "10" represents the maximum of the sequence.
