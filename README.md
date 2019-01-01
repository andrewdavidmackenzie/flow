[![Build Status](https://travis-ci.org/andrewdavidmackenzie/flow.svg?branch=master)](https://travis-ci.org/andrewdavidmackenzie/flow)

# Flow
Flow is a library and cli for the creation and execution of parallel asynchronous, data-driven, programs.

To learn more about the project, the ideas behind it, it's components, how to use it and see some 
examples then jump to [The 'flow' Guide](./guide/src/SUMMARY.md)

## Pre-requisites

You need [Git](https://git-scm.com) to clone the repo.

You need 'make' and a rust toolchain (rustup, cargo, rustc) to build from source.

The 'web' and 'electron' parts require:
* [wasm-bindgen] [Node.js](https://nodejs.org/en/download/) (which comes with [npm](http://npmjs.com))
* 'wasm-bindgen' install with `cargo install wasm-bindgen`

The 'electron' part requires [electron-forge](https://github.com/electron-userland/electron-forge) packaging tool
* You can install it with `npm install -g electron-forge`

## Build and test

With pre-requisites installed, from your command line:

```bash
# Clone this repository
git clone https://github.com/andrewdavidmackenzie/flow.git
# Go into the repository directory
cd flow
# Build and test, including running supplied samples and checking their output is correct
make travis
```

## Run a 'flow'
Run one of the supplied sample flows using:

```cargo run  -- samples/fibonacci```

You should get a fibonacci series output to the terminal, 
followed by an "ERROR" on a panic, that is caused by integer overflow 
when the next number gets too big (don't worry, that's expected)

## License

MIT
