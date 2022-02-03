# `flow_macro`

See also: [Code docs](http://andrewdavidmackenzie.github.io/flow/code/doc/flow_macro/index.html)

Flow is an attribute macro to be used on the structure that provides an implementation for a function (by implementing
the `FlowImpl` trait), so that when compiled for the `wasm32` target, code is inserted to help read the inputs,
help form the outputs and allocate memory (`alloc`) as well as serialize and deserialize the data passed across the 
native/wasm boundary.