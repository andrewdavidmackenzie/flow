## Pre-requisites required to build and test
These are the pre-requisites that are required to build and test 'flow':
* rust toolchain (rustup, cargo, rustc via rustup, etc )
   * nightly (still, due to wasm-bindgen), and stable
   * wasm32-unknown-unknown target for building wasm

For generating JS to wasm bindings:
	```wasm-bindgen-cli```

For building the guide:
	```mdbook``` and its ```mdbook-linkcheck```

## Installing pre-requisites
You have to install rustup, cargo and rust native toolchain yourself, I decided to stop 
short of futzing with people's installed compilers via scripts and Makefile targets.

There is a Makefile target 'config' that will attempt to install remaining dependencies most 
rust developers might not already have.
- ```make config```

That will add the wasm32-unknown-unknown target, install wasm-bindgen-cli, mdbook and mdbook-linkcheck 
using cargo