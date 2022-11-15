## Connection
Connections connect a source of data (via an IO Reference) to a sink of data (via an IO Reference) 
of a compatible [type](types.md):
* `name` (Optional) - an Optional name for the flow. This can be used to help in debugging flows
* `from` = [IO Reference](io_references.md) to the data source that this connection comes from
* `to` = [IO Reference](io_references.md) to a data sink that this connection goes to

### Run-time Semantics
An input IO can be connected to multiple outputs, via multiple connections. 

An output IO can be connected to multiple inputs on other flows or functions via multiple connections.

When the data is produced on the output by a function the data is copied to each destination function
using all the connections that exists from that output.

Data can be buffered at each input of a function.

The order of data arrival at a functions input is the order of creation of jobs executed by that function.
However, that does not guarantee order of completion of the job.

A function cannot run until data is available on all inputs.

Loops are permitted from an output to an input, and are used as a feature to achieve certain behaviours.

When a function runs it produces a result that can contain an output. The result also contains all the 
inputs used to produce any output. Thus input values can be reused by connecting from this "output input-value"
in connections to other processes, or looped back to an input of the same function.

Example, the [fibonacci example](../../flowsamples/fibonacci/root.toml) uses this to define recursion.

```
...
# Loop back the input value #2 from this calculation, to be the input to input #1 on the next iteration
[[connection]]
from = "add/i2"
to = "add/i1"
...
```

### Type Match
For a connection to be valid and used in execution of a flow, the data source must be found,
the data sink must be found and the two must be of compatible DataTypes.

If those conditions are not met, then a connection will be dropped (with an error message output)
and the flow will attempted to be built and executed without it.

### Runtime type conversion of Compatible Types
The flow runtime library implements some type conversions during flow execution, permitting non-identical
types from an output and input to be connected by the compiler, knowing the runtime will handle it.

These are know as `compatible types`. At the moment the following conversions are implemented but more 
maybe added over time:

#### Matching Types
- Type 'T' --> Type 'T'. No conversion required.

#### Generics
- Generic type --> any input. This assumes the input will check the type and handle appropriately.
- Array/Generic type --> any input. This assumes the input will check the type and handle appropriately.
- any output --> Generic type. This assumes the input will check the type and handle appropriately.
- any output --> Array/Generic type. This assumes the input will check the type and handle appropriately.

#### Array Deserialization
- Array/'T' --> 'T'. The runtime will "deserialize" the array and send it's elements one-by-one to the input.
*NOTE* that 'T' maybe any type, including an Array, which is just a special case.
- Array/Array/'T' --> 'T'. The runtime will "deserialize" the array of arrays and send elements one-by-one to the input

#### Array Wrapping
- 'T' --> Array/'T'. The runtime will take the value and wrap it in an array and send that one-element array to the 
  input. Again, 'T' can be any type, including an Array.
- 'T' --> Array/Array/'T'. The runtime will take the value and wrap it in an array in an array and send that 
  one-element array of arrays to the input.

### Default input or output
If a function only has one input or one output, then naming that input/output is optional.
If not names it is referred to as the default input. Connections may connect data to/from this input just
by referencing the function.

Example, the `stdout` context function only has one input and it is not named
```
function = "stdout"
source = "stdout.rs"
docs = "stdout.md"
impure = true

[[input]]
```

and a connection to it can be defined thus:
```
[[connection]]
from = "add"
to = "stdout"
```

### Named inputs
If an input is defined with a name, then connections to it should include the function name and the input name
to define which input is being used.

Example
```
[[connection]]
from = "add"
to = "add/i2"
```

### Selecting an output
When a function runs it produces a set of outputs, producing data on zero or more of it's outputs, all at once.

A connection can be formed from an output to another input by specifying the output's `route` as part of the 
`IO Reference` in the `from` field of the connection.

Example:
```
[[connection]]
from = "function_name/output_name"
to = "stdout"
```

### Selecting sub-structures of an output
As described in [types](types.md), flow supports Json data types. This includes two "container types", namely:
"object" (a Map) and "array".

If an output produces an object, a connection can be formed from an entry of the map (not the entire map) to a
destination input. This allows (say) connecting a function that produces a Map of strings to another function
that accepts a string. This is done extending the `route` used in the `IO Reference` of the `connection` with 
the output name (to select the output) and the key of the map entry (to select just that map entry).

Example: function called "function" has an output named "output" that produces a Map of strings. 
One of those Map entries has the key "key". Then the string value associated with that key is used in the 
connection.
```
[[connection]]
from = "function/output/key"
to = "stdout"
```

Similarly, if the output is an array of values, a single element from the array can be specified in the `connection`
using a numeric subscript.

Example: function called "function" has an output named "output" that produces an array of strings.
Then a single string from the array can be sent to a destination input thus:
```
[[connection]]
from = "function/output/1"
to = "stdout"
```

### Connecting to multiple destinations
A single output can be connected to multiple destinations by creating multiple connections referencing the output.
But, to make it easier (less typing) to connect an output to multiple destinations the `[[connection]]` format
permits specifying more than one `to = "destination"`.

Example
```
[[connection]]
from = "output"
to = ["destination", "destination2"]
```