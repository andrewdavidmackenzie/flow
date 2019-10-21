# `flow_impl`s

See also: [Code docs](../code/doc/flow_impl/index.html)

This crate defines a trait that implementations of flow
'functions' must implement in order for them to be invoked
by the flowrlib (or other) runtime library.

## Derive Macro
Also, in the flow_impl_derive subdirectory a Derive macro
called `FlowImpl` is defined and implemented.

This should be used on the structure that implements the 
function, in order that when compiled for the `wasm32` 
target code is inserted to allocate memory (`alloc`) and
to serialize and deserialize the data passed across the 
native/wasm boundary.

[comment]: <> (TODO add a reference to the code example in implementation.rs)
