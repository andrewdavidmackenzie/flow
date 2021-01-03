# `flowr`

See also: [Code docs](http://andrewdavidmackenzie.github.io/flow/code/doc/flowr/index.html)

`flowr` is a binary run-time for flows build using the `flowrlib` library.

It handles the execution of `Functions` forming a `Flow` according to the defined semantics.

## Flowrlib
`flowrlib` is the library that implements most of the flowr functionality and is used by the 
`flowr` binary. For more details consult it's [README.md](src/lib/README.md)

## Flowruntime
`flowruntime` implements the [flowruntime functions](src/lib/flowruntime/README.md) that
all runtimes for executing flows must provide.