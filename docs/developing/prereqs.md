## Pre-requisites required to build and test
These are the pre-requisites that are required to build and test 'flow':
* rust toolchain (`rustup`, `cargo`, `rustc`, etc )
    * with `wasm32-unknown-unknown` target for compiling to wasm
    * `clippy` for checking coding best practices
* `zmq` (Zero Message Queue) library
* `graphviz` utilities for automatic generation of SVG files for docs

For building the guide:
	```mdbook``` and the ```mdbook-linkcheck``` plug-in

## Installing pre-requisites
You have to install rustup, cargo and rust toolchain yourself.
I decided to stop short of futzing with people's installed toolchains.

There is a Makefile target `config` that will install the other dependencies:
- `make config`

That will add the `wasm32-unknown-unknown` target, `clippy`, `graphviz`, `mdbook` and `mdbook-linkcheck`.
