# flow_impl

The root crate defined a trait that implementations of flow
'functions' must implement in order for them to be invoked
by the flowr (or other) runtime.

## Derive Macro
Also, in the flow_impl_derive subdirectory a Dervice macro
called `FlowImpl` is defined and implemented.

This should be used on the structure that implements the 
function, in order that when compiled for the `wasm32` 
target code is inserted to allocate memory (`alloc`) and
to serialize and deserialize the data passed across the 
native/wasm boundary.

## Example implementation
An example implementation using both of these (the trait and the
derive macro) is shown:

```
extern crate core;
extern crate flow_impl;
extern crate flow_impl_derive;
#[macro_use]
extern crate serde_json;

use flow_impl::implementation::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use serde_json::Value;

#[derive(FlowImpl)]
pub struct Compare;

/*
    A compare operator that takes two numbers (for now) and outputs the comparisons between them
*/
impl Implementation for Compare {
    fn run(&self, mut inputs: Vec<Vec<Value>>) -> (Option<Value>, RunAgain) {
        let left = inputs[0].remove(0).as_i64().unwrap();
        let right = inputs[1].remove(0).as_i64().unwrap();

        let output = json!({
                    "equal" : left == right,
                    "lt" : left < right,
                    "gt" : left > right,
                    "lte" : left <= right,
                    "gte" : left >= right,
                });

        (Some(output), RUN_AGAIN)
    }
}
```