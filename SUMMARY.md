# The `flow` book

[Intro](book/book_intro.md)

# Introduction to `flow`
- [Installing flow](INSTALLING.md)
- [What is `flow`?](book/introduction/what_is_flow.md)
- [Tenets of `flow`](book/introduction/tenets.md)
- [Project Components and Structure](book/introduction/structure.md)
- [The Inspirations for `flow`](book/introduction/inspirations.md)
- [Non-Inspirations](book/introduction/non_inspirations.md)
- [Parallelism](book/introduction/parallelism.md)
- [Status](book/introduction/status.md)

# Your First Flow
- [Your First Flow](book/first_flow/first_flow.md)
- [Running the flow](book/first_flow/implementation.md)
- [Step-by-Step](book/first_flow/step-by-step.md)
- [Debugging your first flow](book/first_flow/debugging.md)

# Defining Flows
- [Defining flows](book/describing/definition_overview.md)
- [Names](book/describing/names.md)
- [Flow IOs](book/describing/ios.md)
- [Process References](book/describing/process_references.md)
- [Function Definitions](book/describing/function_definitions.md)
- [Data Types](book/describing/types.md)
- [Connections](book/describing/connections.md)
- [Flow Libraries](book/describing/flow_libraries.md)
- [Context Functions](book/describing/context_functions.md)
- [Provided Functions](book/describing/provided_functions.md)
- [Programming Methods](book/describing/programming_methods.md)

# `flowrcli's` `context functions`
- [flowr's context functions](flowr/src/bin/flowrcli/context/flowrcli_context_functions.md)
- [Arg functions](flowr/src/bin/flowrcli/context/args/args.md)
  - [Arg get function](flowr/src/bin/flowrcli/context/args/get.md)
- [File functions](flowr/src/bin/flowrcli/context/file/file.md)
  - [File Write function](flowr/src/bin/flowrcli/context/file/file_write.md)
  - [File Read function](flowr/src/bin/flowrcli/context/file/file_read.md)
- [Image manipulation functions](flowr/src/bin/flowrcli/context/image/image.md)
  - [Image buffer](flowr/src/bin/flowrcli/context/image/image_buffer.md)
- [Standard IO functions](flowr/src/bin/flowrcli/context/stdio/stdio.md)
  - [Readline function](flowr/src/bin/flowrcli/context/stdio/readline.md)
  - [Standard Input function](flowr/src/bin/flowrcli/context/stdio/stdin.md)
  - [Standard Output function](flowr/src/bin/flowrcli/context/stdio/stdout.md)
  - [Standard Error function](flowr/src/bin/flowrcli/context/stdio/stderr.md)

# `flowrgui's` `context functions`
- [flowrgui's context functions](flowr/src/bin/flowrgui/context/flowrgui_context_functions.md)
- [Arg functions](flowr/src/bin/flowrgui/context/args/args.md)
  - [Arg get function](flowr/src/bin/flowrgui/context/args/get.md)
- [File functions](flowr/src/bin/flowrgui/context/file/file.md)
  - [File Write function](flowr/src/bin/flowrgui/context/file/file_write.md)
  - [File Read function](flowr/src/bin/flowrgui/context/file/file_read.md)
- [Image manipulation functions](flowr/src/bin/flowrgui/context/image/image.md)
  - [Image buffer](flowr/src/bin/flowrgui/context/image/image_buffer.md)
- [Standard IO functions](flowr/src/bin/flowrgui/context/stdio/stdio.md)
  - [Readline function](flowr/src/bin/flowrgui/context/stdio/readline.md)
  - [Standard Input function](flowr/src/bin/flowrgui/context/stdio/stdin.md)
  - [Standard Output function](flowr/src/bin/flowrgui/context/stdio/stdout.md)
  - [Standard Error function](flowr/src/bin/flowrgui/context/stdio/stderr.md)

# Running Flows
- [Running flows](book/running/running.md)
- [flowc Command Line Arguments](book/running/flowc.md)
- [Passing command line arguments to flows](book/running/arguments.md)
- [Specifying the root file to load](book/running/root_file_selection.md)
- [Standard Input and Output](book/running/stdio.md)
- [Exceptions and Panics](book/running/panics.md)
- [Running Flows using `flowr`](book/running/flowr.md)
- [Running a flow in client/server mode of `flowr`](book/running/client_server.md)
- [Distributed execution of jobs with `flowr` and `flowrex`](book/running/distributed.md)

# Debugging Flows
- [The Debugger](book/debugging/debugger.md)

# The `flowstdlib` Standard Library
- [README](flowstdlib/README.md)
- [control](flowstdlib/src/control/control.md)
    - [compare_switch](flowstdlib/src/control/compare_switch/compare_switch.md)
    - [index](flowstdlib/src/control/index/index.md)
    - [index_f](flowstdlib/src/control/index_f.md)
    - [join](flowstdlib/src/control/join/join.md)
    - [route](flowstdlib/src/control/route/route.md)
    - [select](flowstdlib/src/control/select/select.md)
    - [tap](flowstdlib/src/control/tap/tap.md)
- [data](flowstdlib/src/data/data.md)
    - [accumulate](flowstdlib/src/data/accumulate/accumulate.md)
    - [append](flowstdlib/src/data/append/append.md)
    - [count](flowstdlib/src/data/count/count.md)
    - [duplicate](flowstdlib/src/data/duplicate/duplicate.md)
    - [enumerate](flowstdlib/src/data/enumerate/enumerate.md)
    - [info](flowstdlib/src/data/info/info.md)
    - [ordered_split](flowstdlib/src/data/ordered_split/ordered_split.md)
    - [remove](flowstdlib/src/data/remove/remove.md)
    - [sort](flowstdlib/src/data/sort/sort.md)
    - [split](flowstdlib/src/data/split/split.md)
    - [zip](flowstdlib/src/data/zip/zip.md)
- [fmt](flowstdlib/src/fmt/fmt.md)
    - [reverse](flowstdlib/src/fmt/reverse/reverse.md)
    - [to_json](flowstdlib/src/fmt/to_json/to_json.md)
    - [to_string](flowstdlib/src/fmt/to_string/to_string.md)
- [math](flowstdlib/src/math/math.md)
    - [add](flowstdlib/src/math/add/add.md)
    - [compare](flowstdlib/src/math/compare/compare.md)
    - [divide](flowstdlib/src/math/divide/divide.md)
    - [multiply](flowstdlib/src/math/multiply/multiply.md)
    - [range](flowstdlib/src/math/range.md)
    - [range_split](flowstdlib/src/math/range_split/range_split.md)
    - [sequence](flowstdlib/src/math/sequence.md)
    - [sqrt](flowstdlib/src/math/sqrt/sqrt.md)
    - [subtract](flowstdlib/src/math/subtract/subtract.md)
- [matrix](flowstdlib/src/matrix/matrix.md)
    - [duplicate_rows](flowstdlib/src/matrix/duplicate_rows/duplicate_rows.md)
    - [multiply](flowstdlib/src/matrix/multiply.md)
    - [multiply_row](flowstdlib/src/matrix/multiply_row/multiply_row.md)
    - [transpose](flowstdlib/src/matrix/transpose/transpose.md)
    - [compose_matrix](flowstdlib/src/matrix/compose_matrix/compose_matrix.md)
    
# Examples flows
- [Examples Intro](flowr/examples/README.md)
    - [args](flowr/examples/args/DESCRIPTION.md)
    - [args-json](flowr/examples/args-json/DESCRIPTION.md)
    - [arrays](flowr/examples/arrays/DESCRIPTION.md)
    - [debug-help-string](flowr/examples/debug-help-string/DESCRIPTION.md)
    - [debug-print-args](flowr/examples/debug-print-args/DESCRIPTION.md)
    - [double-connection](flowr/examples/double-connection/DESCRIPTION.md)
    - [factorial](flowr/examples/factorial/DESCRIPTION.md)
    - [fibonacci](flowr/examples/fibonacci/DESCRIPTION.md)
    - [hello-world](flowr/examples/hello-world/DESCRIPTION.md)
    - [line-echo](flowr/examples/line-echo/DESCRIPTION.md)
    - [mandlebrot](flowr/examples/mandlebrot/DESCRIPTION.md)
      - [escapes](flowr/examples/mandlebrot/escapes/escapes.md) provided function
      - [pixel_to_point](flowr/examples/mandlebrot/pixel_to_point/pixel_to_point.md) provided function
    - [pipeline](flowr/examples/pipeline/DESCRIPTION.md)
    - [prime](flowr/examples/prime/DESCRIPTION.md)
    - [primitives](flowr/examples/primitives/DESCRIPTION.md)
    - [reverse-echo](flowr/examples/reverse-echo/DESCRIPTION.md)
    - [router](flowr/examples/router/DESCRIPTION.md)
    - [sequence](flowr/examples/sequence/DESCRIPTION.md)
    - [sequence-of-sequences](flowr/examples/sequence-of-sequences/DESCRIPTION.md)
    - [tokenizer](flowr/examples/tokenizer/DESCRIPTION.md)
    - [two-destinations](flowr/examples/two-destinations/DESCRIPTION.md)

    
--------------------------------------------------------------------------------

# Developing `flow`
- [Developing flow](book/developing/overview.md)
- [Pre-requisites](book/developing/prereqs.md)
- [Building](book/developing/building.md)
- [Crates](book/developing/crates.md)
    - [flowcore](flowcore/README.md)
    - [flowmacro](flowmacro/README.md)
      - [flowc](flowc/README.md)
        - [Flowc Compiler and Parser Test flows](flowc/tests/test-flows/README.md)
    - [flowr](flowr/README.md)
      - [flowrlib](flowr/README.md)
      - [flowrex](flowr/README.md)
      - [flowstdlib](flowstdlib/README.md)
      - [examples](flowr/examples/README.md)
- [Makefile targets](book/developing/make.md)
- [Ways to contribute](book/developing/contributing.md)
    - [Issues](book/developing/issues.md)
    - [PRs](book/developing/prs.md)
    - [Developing an example](flowr/examples/README.md)
    - [Continuous Integration Tests](book/developing/ci.md)
        
# Internals of the `flow` Project
- [Internals of flow](book/internals/overview.md)
    - [Flow Loading](book/internals/flow_loading.md)
    - [Flow Compiling](book/internals/flow_compiling.md)
    - [Flow Execution](book/internals/flow_execution.md)
    - [Flow Execution State Transitions](book/internals/state_transitions.md)

