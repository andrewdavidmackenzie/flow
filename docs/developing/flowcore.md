# `flowcore`

See also: [Code docs](http://andrewdavidmackenzie.github.io/flow/code/doc/flowcore/index.html)

`flowcore` is a library of core structs and traits related to `flow` that are shared between multiple
crates in the `flow`project.

# `Implementation` trait

This is a trait that implementations of flow 'functions' must implement in order for them to be invoked
by the flowrlib (or other) run-time library.

An example of a function implementing the `Implementation` trait can be found in the
docs for [Implementation](http://andrewdavidmackenzie.github.io/flow/code/doc/flowcore/trait.Implementation.html)

# `provider`
This implements a `content provider` that resolves URLs and then gets the content of the url.