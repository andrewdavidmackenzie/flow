# The `flow` book

[Intro](book_intro.md)

# Introduction to `flow`

- [What is `flow`?](docs/introduction/what_is_flow.md)
- [Tenets of `flow`](docs/introduction/tenets.md)
- [Project Components and Structure](docs/introduction/structure.md)
- [The Inspirations for `flow`](docs/introduction/inspirations.md)
- [Non-Inspirations](docs/introduction/non_inspirations.md)
- [Status](docs/introduction/status.md)
- [Flow Programming](docs/introduction/flow_programming.md)

# Your First Flow

- [Your First Flow](docs/first_flow/first_flow.md)
- [Understanding it](docs/first_flow/understanding.md)
- [Real Implementation](docs/first_flow/implementation.md)
- [Step-by-Step](docs/first_flow/step-by-step.md)
- [Debugging your first flow](docs/first_flow/debugging.md)

# Defining Flows
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

# Running Flows
- [Running flows](docs/running/running.md)
- [flowc Command Line Arguments](docs/running/flowc.md)
- [Passing command line arguments to flows](docs/running/arguments.md)
- [Selecting the Context file](docs/running/context_selection.md)
- [Standard Input and Output](docs/running/stdio.md)
- [Exceptions and Panics](docs/running/panics.md)

# Debugging Flows
- [The Debugger](docs/debugging/debugger.md)

# The `flowruntime` Runtime
- [The flowruntime functions](flowr/src/lib/context/README.md)
- [Arg functions](flowr/src/lib/context/args/args.md)
    - [Arg get function](flowr/src/lib/context/args/get.md)
- [File functions](flowr/src/lib/context/file/file.md)
    - [File Write function](flowr/src/lib/context/file/file_write.md)
- [Standard IO functions](flowr/src/lib/context/stdio/stdio.md)
    - [Readline function](flowr/src/lib/context/stdio/readline.md)
    - [Standard Input function](flowr/src/lib/context/stdio/stdin.md)
    - [Standard Output function](flowr/src/lib/context/stdio/stdout.md)
    - [Standard Error function](flowr/src/lib/context/stdio/stderr.md)

# The `flowstdlib` Standard Library
- [flowstdlib](flowstdlib/README.md)
- [control](flowstdlib/control/control.md)
    - [compare_switch](flowstdlib/control/compare_switch/compare_switch.md)
    - [index](flowstdlib/control/index/index.md)
    - [index_f](flowstdlib/control/index_f.md)
    - [join](flowstdlib/control/join/join.md)
    - [route](flowstdlib/control/route/route.md)
    - [select](flowstdlib/control/select/select.md)
    - [tap](flowstdlib/control/tap/tap.md)
- [data](flowstdlib/data/data.md)
    - [accumulate](flowstdlib/data/accumulate/accumulate.md)
    - [append](flowstdlib/data/append/append.md)
    - [buffer](flowstdlib/data/buffer/buffer.md)
    - [count](flowstdlib/data/count/count.md)
    - [duplicate](flowstdlib/data/duplicate/duplicate.md)
    - [duplicate_rows](flowstdlib/data/duplicate_rows/duplicate_rows.md)
    - [info](flowstdlib/data/info/info.md)
    - [multiply_row](flowstdlib/data/multiply_row/multiply_row.md)
    - [remove](flowstdlib/data/remove/remove.md)
    - [sort](flowstdlib/data/sort/sort.md)
    - [split](flowstdlib/data/split/split.md)
    - [transpose](flowstdlib/data/transpose/transpose.md)
    - [zip](flowstdlib/data/zip/zip.md)
- [fmt](flowstdlib/fmt/fmt.md)
    - [reverse](flowstdlib/fmt/reverse/reverse.md)
    - [to_json](flowstdlib/fmt/to_json/to_json.md)
    - [to_string](flowstdlib/fmt/to_string/to_string.md)
- [math](flowstdlib/math/math.md)
    - [add](flowstdlib/math/add/add.md)
    - [compare](flowstdlib/math/compare/compare.md)
    - [divide](flowstdlib/math/divide/divide.md)
    - [multiply](flowstdlib/math/multiply/multiply.md)
    - [sequence](flowstdlib/math/sequence.md)
    - [subtract](flowstdlib/math/subtract/subtract.md)
    - [sqrt](flowstdlib/math/sqrt/sqrt.md)
    
# Sample flows
- [Samples Intro](samples/README.md)
    - [args](samples/args/DESCRIPTION.md)
    - [arrays](samples/arrays/DESCRIPTION.md)
    - [factorial](samples/factorial/DESCRIPTION.md)
    - [fibonacci](samples/fibonacci/DESCRIPTION.md)
    - [hello-world](samples/hello-world/DESCRIPTION.md)
    - [mandlebrot sample](samples/mandlebrot/DESCRIPTION.md)
    - [matrix multiplication](samples/matrix_mult/DESCRIPTION.md)
    - [pipeline](samples/pipeline/DESCRIPTION.md)
    - [prime](samples/prime/DESCRIPTION.md)
    - [primitives](samples/primitives/DESCRIPTION.md)
    - [range](samples/range/DESCRIPTION.md)
    - [range-of-ranges](samples/range-of-ranges/DESCRIPTION.md)
    - [reverse-echo](samples/reverse-echo/DESCRIPTION.md)
    - [router](samples/router/DESCRIPTION.md)
    - [tokenizer](samples/tokenizer/DESCRIPTION.md)
    
--------------------------------------------------------------------------------

# Developing `flow`
- [Developing flow](docs/developing/overview.md)
- [Pre-requisites](docs/developing/prereqs.md)
    - [flowcore](flowcore/README.md)
    - [flow_impl_derive](flow_impl_derive/README.md)
    - [flowc](flowc/README.md)
        - [Test flows](flowc/tests/test-flows/README.md)
            - [hello-world](flowc/tests/test-flows/hello-world/hello-world.md)
            - [line-echo](flowc/tests/test-flows/line-echo/line-echo.md)
            - [print-args](flowc/tests/test-flows/print-args/print-args.md)
            <!--- TODO add Markdown docs for other test flows explaining what they test -->
    - [flowr](flowr/README.md)
    - [flowrlib](flowr/src/lib/README.md)
    - [flowstdlib](flowstdlib/README.md)
    - [flowruntime](flowr/src/lib/context/README.md)
    - [samples](samples/README.md)
- [Makefile targets](docs/developing/make.md)
- [Ways to contribute](docs/developing/contributing.md)
    - [Issues](docs/developing/issues.md)
    - [PRs](docs/developing/prs.md)
    - [Developing a sample](samples/README.md)
    - [Continuous Integration Tests](docs/developing/ci.md)
        
# Internals of the `flow` Project
- [Internals of flow](docs/internals/overview.md)
    - [Flow Loading](docs/internals/flow_loading.md)
    - [Flow Compiling](docs/internals/flow_compiling.md)
    - [Flow Execution](docs/internals/flow_execution.md)

