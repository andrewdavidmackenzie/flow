# `flowcore`

See also: [Code docs](http://andrewdavidmackenzie.github.io/flow/code/doc/flowcore/index.html)

`flowcore` is a library of core structs and traits related to flow that is shared between multiple flow
crates, and separate to avoid a cyclic dependency.

# `flow_impl`

See also: [Code docs](http://andrewdavidmackenzie.github.io/flow/code/doc/flow_impl/index.html)

This is a trait that implementations of flow 'functions' must implement in order for them to be invoked
by the flowrlib (or other) run-time library.

An example of a function implementing the `Implementation` trait can be found in the
docs for [`Implementation`](http://andrewdavidmackenzie.github.io/flow/code/doc/flowcore/trait.Implementation.html)