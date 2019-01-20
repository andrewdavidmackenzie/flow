## Connection
Connections connect a source of data (via an IO Reference) to a sink of data (via an IO Reference) 
of a compatible [type](types.md):
* `name` [Optional] - an Optional name for the flow
* `from` = [IO Reference](io_references.md) to the Data source that this connection comes from
* `to` = [IO Reference](io_references.md) to a Data sink that this connection goes to

The type of a data flow is inferred from the two IOs that it connects (after checking the types match)

### Runtime Semantics
An input IO can receive data from (i.e. be connected to) multiple outputs. 

The first to arrive is the one that will fulfil the input the connection connects to and the
destination will execute with that input value.

An output IO can be connected to multiple inputs on other values/flows/functions via multiple 
connections.

When the data is produced on the output by the Data source the data is copied to each 
Data sink via each connection that exists.

An output can only be produced when the data can be sent to *all* connected destinations, 
avoiding any data loss or need for buffering. The sender is blocked until all destinations are
available.

A runnable maybe blocked on output by other "busy" inputs, thus inputs are not overwritten 
but queued up with backpressure.

A function or value will not be executed until all inputs are available and it can sent its 
output to all connected destinations.

Loops are permitted from an output to an input, and are used as a feature to achieve certain behaviours.

### Type Match
For a connection to be valid and used in execution of a flow, the data source must be found,
the data sink must be found and the two must be of matching DataTypes.

If those conditions are not met, then a connection will be dropped (with an error message output)
and the flow will attempted to be built and executed without it.
