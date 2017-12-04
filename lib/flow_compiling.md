# Flow Compiling

Start with the hierarchical definition of flow program as produced by the loading stage.

## Value and Function Tables
Build a table of values and functions with their correct routes.
  
## Connection Reducing
Build a flat table of connections.

Pass through the connection table.
For every connection that starts at a flow:
- Look through all other connections and for each one that ends at where this flow starts
  replace the connection's destination with this connections destination.
- Delete this connection

? Maybe have to do multiple times?

When done there should be no connections starting at flows.
Any connections left that end at flows, are unconnected and can be dropped in the pruning
stage below.

### Pruning
Drop the following combinations, with warnings:
- connection that ends at a flow, as none else connect to it.
- values that don't have connections from them.
- values that have only outputs and are not initialized.
- functions that don't have connections from at least one output.
- functions that don't have connections to all their inputs.

