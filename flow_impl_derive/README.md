# `flow_impl_derive`

See also: [Code docs](../code/doc/flow_impl_derive/index.html)

FlowImpl is a derive macro to be used on the structure that implements a function (by implementing
the `FlowImpl` trait), so that when compiled for the `wasm32` target, code is inserted to allocate memory (`alloc`) and
to serialize and deserialize the data passed across the native/wasm boundary.