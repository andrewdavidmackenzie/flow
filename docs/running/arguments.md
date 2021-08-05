### Passing Command Line Arguments
Arguments are passed to the flow being executed by `flowc` by placing them after the flow name in 
the execution string (either using `cargo run -p flowc` or `flowc` directly). 
e.g. `cargo run -p flowc -- samples/mandlebrot mandel.png 4000x3000 -1.20,0.35 -1,0.20`

The `flowruntime` provides a function called `args` that can be included in a flow definition 
that fetches these arguments, allowing them to then be processed in the flow like any other inputs.

Include the `args` function in your flow:
```toml
[[process]]
alias = "args"
source = "lib://flkowr/args/get"
```

Then create a connection from the desired output of `args` to another function:
```toml
[[connection]]
from = "function/args/2"
to = "function/parse_bounds/input"
```