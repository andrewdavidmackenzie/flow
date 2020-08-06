# Summary

[README](README.md)

--------------------------------------------------------------------------------

- [Introduction](docs/introduction/introduction.md)
    - [What is flow](docs/introduction/what_is_flow.md)
    - [Tenets of flow](docs/introduction/tenets.md)
    - [Project Components](docs/introduction/components.md)
    - [The Inspirations for flow](docs/introduction/inspirations.md)
    - [Non-Inspirations](docs/introduction/non_inspirations.md)
    - [Status](docs/introduction/status.md)
    - [Flow Programming](docs/introduction/flow_programming.md)

- [Your First Flow](docs/first_flow/first_flow.md)
    - [Understanding it](docs/first_flow/understanding.md)
    - [Real Implementation](docs/first_flow/implementation.md)
    - [Step-by-Step](docs/first_flow/step-by-step.md)
    - [Debugging your first flow](docs/first_flow/debugging.md)

- [Defining flows](docs/describing/definition_overview.md)
    - [Names](docs/describing/names.md)
    - [Flow IOs](docs/describing/ios.md)
    - [Process References](docs/describing/process_references.md)
    - [Function Definitions](docs/describing/function_definitions.md)
    - [Types](docs/describing/types.md)
    - [Connections](docs/describing/connections.md)
    - [IO References](docs/describing/io_references.md)
    - [Complete Feature List](docs/describing/features.md)
    - [Flow Libraries](docs/describing/flow_libraries.md)

- [Running flows](docs/running/running.md)
    - [flowc Command Line Arguments](docs/running/flowc.md)
    - [Passing command line arguments to flows](docs/running/arguments.md)
    - [Selecting the Context file](docs/running/context_selection.md)
    - [Standard Input and Output](docs/running/stdio.md)
    - [Exceptions and Panics](docs/running/panics.md)

- [The flowruntime functions](flowruntime/README.md)
    - [Arg functions](flowruntime/args/args.md)
        - [Arg get function](flowruntime/args/get.md)
    - [File functions](flowruntime/file/file.md)
        - [File Write function](flowruntime/file/file_write.md)
    - [Standard IO functions](flowruntime/stdio/stdio.md)
        - [Readlin function](flowruntime/stdio/readline.md)
        - [Standard Input function](flowruntime/stdio/stdin.md)
        - [Standard Output function](flowruntime/stdio/stdout.md)
        - [Standard Error function](flowruntime/stdio/stderr.md)

- [Test Samples](flowc/tests/samples/README.md)
    - [hello-world](flowc/tests/samples/hello-world/hello-world.md)
    - [line-echo](flowc/tests/samples/line-echo/line-echo.md)
    - [print-args](flowc/tests/samples/print-args/print-args.md)

- [Samples](samples/README.md)
    - [args sample](samples/args/DESCRIPTION.md)
    - [arrays sample](samples/arrays/DESCRIPTION.md)
    - [factorial sample](samples/factorial/DESCRIPTION.md)
    - [fibonacci sample](samples/fibonacci/DESCRIPTION.md)
    - [hello-world sample](samples/hello-world/DESCRIPTION.md)
    - [matrix multiplication sample](samples/matrix_mult/DESCRIPTION.md)
    - [pipeline sample](samples/pipeline/DESCRIPTION.md)
    - [prime sample](samples/prime/DESCRIPTION.md)
    - [primitives sample](samples/primitives/DESCRIPTION.md)
    - [range](samples/range/DESCRIPTION.md)
    - [range-of-ranges](samples/range-of-ranges/DESCRIPTION.md)
    - [reverse-echo sample](samples/reverse-echo/DESCRIPTION.md)
    - [tokenizer sample](samples/tokenizer/DESCRIPTION.md)

<!---
- WIP Samples
    - [router sample](samples/router/DESCRIPTION.md) (WIP)
    - [mandlebrot-world sample](samples/mandlebrot/DESCRIPTION.md) (WIP)
-->

- [Known `flow` libraries](docs/libraries/known-libraries.md)
    - [The flow standard library](flowstdlib/README.md)

--------------------------------------------------------------------------------

- [Developing flow](docs/developing/overview.md)
    - [Pre-requisites](docs/developing/prereqs.md)
    - [Project Structure](docs/developing/structure.md)
        - [flow_impl](flow_impl/README.md)
        - [flow_impl_derive](flow_impl_derive/README.md)
        - [flowc](flowc/README.md)
        - [flowr](flowr/README.md)
        - [flowrlib](flowrlib/README.md)
        - [flowstdlib](flowstdlib/README.md)
        - [flowide](flowide/README.md)
        - [provider](provider/README.md)
        - [flowruntime](flowruntime/README.md)
        - [samples](samples/README.md)
    - [Makefile targets](docs/developing/make.md)
    - [Ways to contribute](docs/developing/contributing.md)
        - [Issues](docs/developing/issues.md)
        - [PRs](docs/developing/prs.md)
        - [Developing a sample](samples/sample_development.md)
        - [Continuous Integration Tests](docs/developing/ci.md)
        
- [Internals of flow](docs/internals/overview.md)
    - [Flow Loading](docs/internals/flow_loading.md)
    - [Flow Compiling](docs/internals/flow_compiling.md)
    - [Flow Execution](docs/internals/flow_execution.md)

