# Flow Compiling

From the hierarchical definition of a flow program as produced by the loading stage:

## Connection Reducing
Build a flat table of connections.

For every connection that ends at a flow:
- Look through all other connections and for each one that starts where this flow starts:
  - Replace the connection's destination with this connections destination.
  - Delete this connection

When done there should be no connections end at flows.
Any connections left that starts at a flow, is unconnected and can be deleted.

## Value and Function Tables
Build a table of values and functions.
  
### Pruning Value and Function Tables
Drop the following combinations, with warnings:
- values that don't have connections from them.
- values that have only outputs and are not initialized.
- functions that don't have connections from at least one output.
- functions that don't have connections to all their inputs.