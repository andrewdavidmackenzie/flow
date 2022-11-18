## Context Functions
Each flow runner application can provide a set of functions (referred to as `context functions`) to flows for 
interacting with the execution environment.

They are identified by a flow defining a process reference that uses the `context://` Url scheme.
(see [process references](process_references.md) for more details).

In order to compile a flow the compiler must be able to find the definition of the function.

In order to execute a flow the flow runner must either have an embedded implementation of the function or
know how to load one.

Different runtimes may provide different functions, and thus it is not guaranteed that a function is present 
at runtime.

### Completion of Functions
Normal "pure" functions can be executed any number of times as their output depends only on the inputs and the
(unchanging) implementation. They can be run any time a set of inputs is available.

However, a context function may have a natural limit to the number of times it can be ran during the execution of
a flow using it. An example would be a function that reads a line of text from a file. It can be ran as many times
as there are lines of text in the file, then it will return End-Of-File and a flag to indicate to the flow runtime
that it has "completed" should not be invoked again.

If this was not done, as the function has no inputs, it would always be available to run, and be executed 
indefinitely, just to return EOF each time. 

For this reason, each time a function is run, it returns a "run me again" flag that the runtime uses to determine
if it has "completed" or not. If it returns true, then the function is put into the "completed" state and it will
never be run again (during that flow's execution)

### Specifying the Context Root
At compile time the compiled must know which functions are available and their definitions.

Since it is the flow runner that provides the implementations and knows their definitions, it must make these
discoverable and parseable by the compiler as a set of function definition files.

This is done by specifying to the `flowc` compiled what is called the `context root` or the root folder of 
where the targeted runtime's context functions reside.

### Context Function Process References
A reference to a `context function` process (in this case it is always a function) such as STDOUT is of the form:
```
[[process]]
source = "context://stdio/stdout"
```

The `context://` Url scheme identifies it is a `context function` and it's definition should be sought below
the `Context Root`. The rest of the Url specifies the location under the `Context Root` directory (once found).

### Example
The `flow` project directory structure is used in this example, with `flow` located at `/Users/me/flow` and 
`flow` in the users `$PATH`.

The fibonacci sample flow is thus found in the `/Users/me/flow/flowsamples/fibonacci` directory.

The `flowr` flow runner directory is thus at `/Users/me/flow/flowr`. Within that folder flowr provides a set of 
context function definitions for a Command Line Interface (CLI) implementation in the `src/cli` sub-directory.

If in the root directory of the `flow` project, using relative paths, the sample flow can be compiled and 
run using the `-C, --context_root <CONTEXT_DIRECTORY>` option to `flowc`:
```
> flowc -C flowr/src/cli flowsamples/fibonacci
```

The `flowc` compiler sees the `"context://stdio/stdout"` reference. It has been told that the `Context Root` is
at `flowr/src/cli` so it searches for (and finds) a function definition file at `flowr/src/cli/stdio/stdout/stdout.toml`
using the alrgorithm described in [process references](process_references.md).