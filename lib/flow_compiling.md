# Flow Compiling

## Load the Flow definition
Read in the hierarchical definition, recursively until all is loaded.
- this will build the connections between values, functions, input and oututs
  using the unaliased routes to functions and subflows....
  
## Reducing
Build a table of values and functions with their correct routes.

Build a flat table of connections.

Pass therough the connection table, collapsing any connections that don't start or end 
at a value or function, eliminating all the intermediate connection points are flow entry
or exit boundaries, until we have the minimal connection set.

When multiple connections have a single source as the source, they should all be connected,
avoid lossing the initial connection when collapsing first two and so second connection cannot
use it.

Prune any connection that doesn't originate or end at a value or function in the tables.
