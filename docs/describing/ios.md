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