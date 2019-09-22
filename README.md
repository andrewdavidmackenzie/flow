[![Build Status](https://travis-ci.org/andrewdavidmackenzie/flow.svg?branch=master)](https://travis-ci.org/andrewdavidmackenzie/flow)

# Welcome!
Welcome to my project called `flow` for defining and running parallel, data-dependency-driven 'programs'.

It currently consists of libraries for compiling (`flowclib`) and running (`flowrlib`) flows as 
well as command line binaries (`flowc` and `flowr`) that use the libraries. 

Those libraries do not interact with the host system and can be compiled to WebAssembly for other types of applications, such as experiments I have started (`ide` and `ide-native`) on using them to create a type of IDE for flows.
 
 The project is in its early stages and the current implementation is being used by me as a way
 to clarify my ideas and also as a way to continue to learn rust.
 
 Learn more about what it is and why I created it by reading the [Introduction section](docs/introduction/introduction.md), 
 or dive right in and see if you can understand [Your First Flow](docs/first_flow/first_flow.md)
 with zero previous knowledge and just your programmer's intuition.
 
These docs form part of the ['flow' Guide](http://andrewdavidmackenzie.github.io/flow/) published on GitHub.io using gh-pages
which you can read.

## Pre-requisites
You need [Git](https://git-scm.com) to clone the repo.

You need `make` and a rust toolchain (rustup, cargo, rustc) to build from source.

There is a make target `config` to install pre-requisites, so try `make config`.

Some crates depend on `open-ssl` and so you may need to install SSL Development libraries (to get header files) 
to be able to compile them. This was fixed with `sudo apt install libssl-dev` on Linux boxes (Raspberry Pi)
where I found the problem. Seems that `brew install openssl` should do it for Mac.

The `ide` module requires:
* [wasm-bindgen] [Node.js](https://nodejs.org/en/download/) (which comes with [npm](http://npmjs.com))
* 'wasm-bindgen' install with `cargo install wasm-bindgen`

## Build and test

With pre-requisites installed, from your command line:

```bash
# Clone this repository
git clone https://github.com/andrewdavidmackenzie/flow.git
# Go into the repository directory
cd flow
# Build and test, including building and testing docs and guide and running supplied samples and checking their output is correct
make
```

## Run a 'flow'
Run the [first flow](http://andrewdavidmackenzie.github.io/flow/first_flow/first_flow.html) sample flow using:

```cargo run -p flowc -- samples/first```

You should get a series of numbers output to the terminal, 
followed by an "ERROR" on a panic, that is caused by integer overflow 
when the next number gets too big (don't worry, that's expected).

The [first flow](http://andrewdavidmackenzie.github.io/flow/first_flow/first_flow.html) section of the guide explains
what that sample does and walks you through it.

## Make docs or guide changes
As all "guide" content must be under the `docs` folder, I currently have a make target to copy markdown files 
across from other folders (preserving directory structure and relative links between them) under `docs`. 
So, if you make changes to markdown docs, or the guide's `SUMMARY.md` index file, then best to 
run `make doc` (`make travis` depends on this target so it is done for you) before pushing and sending your PR. 

After a PR is merged, and the build of the modifed `master` branch succeeds, the guide is rebuilt with `mdbook` and
 the resulting html is deployed to Github Pages, which can be viewed [here](http://andrewdavidmackenzie.github.io/flow/)

## License

MIT

## Project structure

The Project is structured into a number of sub-folders, many of which aare also rust projects with their own 
`Cargo.toml` file. There is a "workspace manifest" `Cargo.toml` in the root that includes many of them, but not
all.

see the [Structure section](docs/developing/structure.md) of the Guide.
