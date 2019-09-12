[![Build Status](https://travis-ci.org/andrewdavidmackenzie/flow.svg?branch=master)](https://travis-ci.org/andrewdavidmackenzie/flow)

# Flow
Flow is a library and cli for the creation and execution of parallel, asynchronous, data-driven programs.

To learn more about the project, the ideas behind it, it's components, how to use it and see some 
examples then jump to [The 'flow' Guide](http://andrewdavidmackenzie.github.io/flow/)

## Pre-requisites

You need [Git](https://git-scm.com) to clone the repo.

You need 'make' and a rust toolchain (rustup, cargo, rustc) to build from source.

There is a make target `config` to install pre-requisites, so try `make config`.

Some crates depend on `open-ssl` and so you may need to install SSL Development libraries (to get header files) 
to be able to compile them. This was fixed with `sudo apt install libssl-dev` on Linux boxes (Raspberry Pi)
where I found the problem. Seems that `brew install openssl` should do it for Mac.

The 'ide' module requires:
* [wasm-bindgen] [Node.js](https://nodejs.org/en/download/) (which comes with [npm](http://npmjs.com))
* 'wasm-bindgen' install with `cargo install wasm-bindgen`

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
Run the [first flow](http://andrewdavidmackenzie.github.io/flow/first_flow/first_flow.html) sample flow using:

```cargo run --bin flowc -- samples/first```

You should get a series of numbers output to the terminal, 
followed by an "ERROR" on a panic, that is caused by integer overflow 
when the next number gets too big (don't worry, that's expected).

The [first flow](http://andrewdavidmackenzie.github.io/flow/first_flow/first_flow.html) section of the guide explains
what that sample does and walks you through it.

## Make docs or guide changes
As all "guide" content must be under the `guide/src` folder, I currently have a make target to copy markdown files 
across from other folder (preserving directory structure and relative links between them) under `guide/src`. 
So, if you make changes to markdown docs, or the guide's `Summary.md` index file, then best to 
run `make guide` (`make travis` depends on this target so it is done for you) before pushing and sending your PR. 
I need to fix the Github Pages deploy of the guide that is built with `mdbook`.

## License

MIT

## Project structure

The Project is structured into a number of sub-folders, many of which aare also rust projects with their own 
`Cargo.toml` file. There is a "workspace manifest" `Cargo.toml` in the root that includes many of them, but not
all.

see the [Structure section](guide/src/developing/structure.md) of the Guide.
