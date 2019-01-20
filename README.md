[![Build Status](https://travis-ci.org/andrewdavidmackenzie/flow.svg?branch=master)](https://travis-ci.org/andrewdavidmackenzie/flow) [![Waffle.io - Issues in progress](https://badge.waffle.io/andrewdavidmackenzie/flow.png?label=in%20progress&title=In%20Progress)](http://waffle.io/andrewdavidmackenzie/flow)

# Flow
Flow is a library and cli for the creation and execution of parallel, asynchronous, data-driven programs.

To learn more about the project, the ideas behind it, it's components, how to use it and see some 
examples then jump to [The 'flow' Guide](./guide/src/SUMMARY.md)

## Pre-requisites

You need [Git](https://git-scm.com) to clone the repo.

You need 'make' and a rust toolchain (rustup, cargo, rustc) to build from source.

Some crates depend on `open-ssl` and so you may need to install SSL Development libraries (to get header files) 
to be able to compile them. This was fixed with `sudo apt install libssl-dev` on Linux boxes (Raspberry Pi)
where I found the problem. Seems that `brew install openssl` should do it for Mac.

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
Run the [first flow](guide/src/first_flow/first_flow.md) sample flow using:

```cargo run  -- samples/first```

You should get a series of numbers output to the terminal, 
followed by an "ERROR" on a panic, that is caused by integer overflow 
when the next number gets too big (don't worry, that's expected).

The [first flow](guide/src/first_flow/first_flow.md) section of the guide explains
what that sample does and walks you through it.

## License

MIT
