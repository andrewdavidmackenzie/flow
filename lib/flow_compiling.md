# Flow Compiling

From the hierarchical definition of a flow program as produced by the loading stage:

## Connection Reducing
Build a flat table of connections.

Pass through the connection table.
For every connection that starts at a flow:
- Look through all other connections and for each one that ends at where this flow starts
  replace the connection's destination with this connections destination.
- Delete this connection

When done there should be no connections starting at flows.
Any connections left that ends at a flow, is unconnected and can be dropped.

## Value and Function Tables
Build a table of values and functions with their correct routes.
  
### Pruning Value and Function Tables
Drop the following combinations, with warnings:
- values that don't have connections from them.
- values that have only outputs and are not initialized.
- functions that don't have connections from at least one output.
- functions that don't have connections to all their inputs.

