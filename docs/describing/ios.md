## IO
IOs produce or consume data of a specific type, and are where data enters/leaves a flow/value/function.

* `name` - the IO Reference that is used to identify it in connections to/from it
* `type` (optional) - An optional [Data type](types.md) for this IO
* `depth` (optional) - An optional specification of the number of entries that must be "queued up" at this input 
before it is considered "available" (default = 1)

### Using `depth`
Some functions may require multiple values from a stream of values to be able to execute 
(e.g. a stream of input coordinates of line starts and ends, to a function to calculate 
line lengths). In this case the input can be defined with a `depth` of 2, and the function will not
be run until two values are available. It will then run and produce one output.

### Initializing an IO
An IO maybe initialized with a value, in one of two ways:
* `once` - the value is inserted into the IO on startup only and there after it will remain empty if a value is not 
sent to it from a Process
* `always` - the value will be inserted into the IO each time it is empty, of there is not a value already
sent from a process.

When a process only has one input, and it is not named, then you can refer to it by the name
`default` for the purposes of specifying an initializer

Eamples:
```toml
[[process]]
alias = "print"
source = "lib://flowruntime/stdio/stdout"
input.default = {once = "Hello World!"}
```

```toml
[[process]]
alias = "second-start"
source = "lib://flowstdlib/fmt/to_number"
input.default = {always = "2"}
```