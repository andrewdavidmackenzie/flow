## IO
IOs produce or consume data of a specific type, and are where data enters/leaves a flow or function (more generally 
referred to as "processes").

* `name` - used to identify an input or output in connections to/from it
* `type` (optional) - An optional [Data type](types.md) for this IO

### Default inputs and outputs
If a function only has one input or one output, then naming that input/output is optional.
If not named, it is referred to as the default input. Connections may connect data to/from this input/output just
by referencing the function.

### Generic Inputs or Outputs
If an input or output has no specific [Data type](types.md) specified, then it is considered `generic` and can 
take inputs of any type. What the function does, or what outputs it produces, may vary depending on the input
type at runtime and should be specified by the implementor of the function and understood by the flow programmer
using it.

Example: A print function could accept any type and print out some human readable representation of all of them.

Example: An `add` function could be overloaded and if provided two numbers it would sum them, but if provided
two strings it could concatenate them.
