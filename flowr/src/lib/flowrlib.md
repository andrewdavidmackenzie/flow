# `flowrlib`

See also: [Code docs](http://andrewdavidmackenzie.github.io/flow/code/doc/flowrlib/index.html)

This is the rust run-time library for flow execution. This will be linked with other code to produce a flow
 runner, such as the `flowr` command line runner.

It is responsible for reading a flow definition in a `Manifest` file, loading the required libraries 
from `LibraryManifest` files and then coordinating the execution by dispatching `Jobs` to be executed 
by `Function` `Implementations`, providing them the `Inputs` required to run and gathering the `Outputs` produced 
and passing those `Outputs` to other connected `Functions` in the network of `Functions`.
