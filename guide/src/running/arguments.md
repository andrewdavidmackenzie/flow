### Passing Command Line Arguments
Arguments are passed to the flow being executed by `flowc` by placing them after the flow name in 
the execution string (either using `cargo run` or `flowc` directly). 
e.g. `cargo run -- samples/mandlebrot mandel.png 4000x3000 -1.20,0.35 -1,0.20`

The flow runtime provides a function called `args` that can be included in a flow definition 
that fetches these arguments, allowing them to then be processed in the flow like any other inputs.

Include the `args` function in your flow:
```
[[process]]
alias = "args"
source = "lib://flkowr/args/get.toml"
```

Then create a connection from the desired output of `args` to another function:
```
[[connection]]
from = "function/args/2"
to = "function/parse_bounds/input"
```