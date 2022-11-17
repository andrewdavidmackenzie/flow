## Connection
Connections connect a source of data (via an IO Reference) to a sink of data (via an IO Reference) 
of a compatible [type](types.md) within a flow.
* `name` (Optional) - an Optional name for the flow. This can be used to help in debugging flows
* `from` = IO Reference to the data source that this connection comes from
* `to` = IO Reference to a data sink that this connection goes to

### Connections at multiple level in flow hierarchy
A flow is a hierarchy from the root flow down, including functions and sub-flows (collectively sub-processes).

Connections are defined within each flow or sub-flow from a source to a destination.

Within a flow sources include:
- an input of this flow
- an output from one of the sub-processes

and destinations include
- an input of one of the sub-processes
- an output of this flow

A connection may be defined with multiple destinations and/or there maybe multiple connections a one source or to a 
destination.

### Connection "branching"
Within a sub-flow there may exist a connection to one of it's outputs, as a destination.
At the next level up in the flow hierarchy that sub-flow output becomes a possible source for connections
defined at that level.

Thus a single connection originating at a single source in the sub-flow may "branch" into multiple connections, 
reaching multiple destinations.

### Connection Gathering and Collapsing
When a flow is compiled, sources of data (function outputs) are followed through the through layers of 
sub-flows/super-flow definition of the flow hierarchy and the resulting "tree" of connections to be eventually 
connected (possibly branching to become multiple connections) to destination(s).

The chain of connections involved in connecting a source to each of the destinations is 
"collapsed" as part of the compilation process, to leave a single connection from the source to each of the destinations. 

### Connection Optimizing
Thru flow re-use, come connections may end up not reaching any destination. The compiler optimizes these connections
away by dropping them.

If in the process of dropping dead connections a function ends up not having any output and/or input (for "pure
functions) it maybe removed, and an error or warning reported by the compiler.

### IO References
An IO Reference uniquely identifies an Input/Data-source (flow/function) or an Output/Data-sink in the flow
hierarchy.

If any flows or functions defined in other files are referenced with an alias, then it should be
used in the IO references to inputs or outputs of that referenced flow/function.

Thus valid IO reference formats to use in connections are:

#### Data sinks
- `input/{input_name}` (where input is a keyword and thus a sub-flow cannot be named `input` or `output`)
- `{sub_process_name}/{output_name}` or `{sub_process}` for the default output

Where `sub_process_name` is a `process` referenced in this flow, and maybe a function or a sub-flow.
The reference use the process's name (if the process was not given an alias when referenced) or it's alias.

#### Data sinks
- `output/{output_name}` (where output is a keyword and thus a sub-flow cannot be named `input` or `output`)
- `{sub_process_name}/{input_name}` or `{sub_process}` for the default input

### Selecting parts of a connection's value
A connection can select to "connect" only part of the data values passed on the source of the connection.
See below [Selecting sub-structures of an output](#selecting-sub-structures-of-an-output) for more details.

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

By not specifying the data type on intermediary connections thru the flow hierarchy, the flow author can enable
connections that are not constrained by the intermediate inputs/outputs used and those types are not need to be 
known when the flow is being authored. In this case the type check will pass on the intermediate connections to 
those "generic" inputs our output.

However, once the connection chain is collapsed down to one end-to-end connection, the source and destination 
types must also pass the type check. This includes intermediate connections that may select part of the value.

Example
- Subflow 1 has a connection: A function `series` with default output Array/Number --> Generic output of the subflow
  - The destination of the connection is generic and so the intermediate type check passes
- Root flow (which contains Subflow 1) as a connection: Generic output of the subflow --> Function `add` input `i1`
  (which has a data type `Number`) that includes selection of an element of the array of numbers `/1`
  - The source is generic, so the intermediate type check passes
- A connection chain is built from the `series` output thru the intermediate connection to the `add` function input `i1`
- The connection chain is collapsed to a connection from the Array element of index 1 of the `series` function's 
output to the `add` functions input `i1`
- The `from` and `to`types of this collapsed connection are both `Number` and so the type check passes

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