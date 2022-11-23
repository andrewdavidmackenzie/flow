# `flowr`

See also: [Code docs](http://andrewdavidmackenzie.github.io/flow/code/doc/flowrex/index.html)

`flowrex` is a binary for the execution of flow jobs, dispatched over the network by `flowr` or some other
flow runner application that runs a `coordinator` (via the `flowrlib` library).

You can find more details about how to use it in running flows in the [distributed](../running/distributed.md) section.

## features
These are the conditionally compiled features of `flowr`:
- default - "flowstdlib" (dependency as a feature) to link `flowstdlib` natively. Deactivating this would
allow use of `flowstdlib` via it's WASM function implementations (use of `-L` option required)