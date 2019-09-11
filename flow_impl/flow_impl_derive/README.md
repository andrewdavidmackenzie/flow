# FlowImpl derive macro
This macro should be used on the structure that implements a 
function, in order that when compiled for the `wasm32` 
target code is inserted to allocate memory (`alloc`) and
to serialize and deserialize the data passed across the 
native/wasm boundary.

See also flow_impl, flowc and flowr crates.