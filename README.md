[![Build Status](https://travis-ci.org/andrewdavidmackenzie/flow.svg?branch=master)](https://travis-ci.org/andrewdavidmackenzie/flow)
[![codecov.io](https://codecov.io/gh/andrewdavidmackenzie/flow/coverage.svg?branch=master)](https://codecov.io/gh/andrewdavidmackenzie/flow/branch/master)
# Welcome!
Welcome to my project called `flow` for defining and running parallel, data-dependency-defined 'programs'.

It currently consists of libraries for compiling (`flowclib`) and running (`flowrlib`) flows as 
well as command line binaries (`flowc` and `flowr`) that use the libraries. 

Those libraries do not interact with the host system and can be compiled to WebAssembly for other types of applications, 
such as experiments I have started (`flowide`) on using them to create a type of IDE for flows.
 
The project is in its early stages and the current implementation is being used by me as a way to clarify my ideas 
and also as a way to continue to learn rust.
 
Learn more about what it is and why I created it by reading the [Introduction section](docs/introduction/introduction.md), 
or dive right in and see if you can understand [Your First Flow](docs/first_flow/first_flow.md) with zero previous knowledge and just your programmer's intuition.
 
This README.md forms part of the ['flow' Guide](http://andrewdavidmackenzie.github.io/flow/) published on GitHub.io 
using gh-pages which you can read.

## Pre-requisites
You need [Git](https://git-scm.com) to clone the repo.

You need `make` and a rust toolchain (rustup, cargo, rustc) to build from source.

Some crates depend on `open-ssl` and so you may need to install SSL Development libraries (to get header files) 
to be able to compile them. This was fixed with `sudo apt install libssl-dev` on Linux boxes (Raspberry Pi)
where I found the problem. Seems that `brew install openssl` should do it for Mac.

The `ide` module requires:
* [Node.js](https://nodejs.org/en/download/) (which includes [npm](http://npmjs.com))
* 'wasm-bindgen' install with `cargo install wasm-bindgen` 

There is a make target `config` to install pre-requisites (e.g. mdbook and mdbook-linkcheck), so try `make config` *after* cloning the repo (see below).

## Clone the repo
From your command line:

```bash
# Clone this repository
git clone https://github.com/andrewdavidmackenzie/flow.git
```

## Build and test
With pre-requisites installed, from your command line:

```bash
# Build and test, including building and testing docs and guide and running supplied samples and checking their output is correct
make
```

## Run your first 'flow'
Run the 'first' sample flow using:

```cargo run -p flowc -- samples/first```

You should get a series of numbers output to the terminal.

The [first flow](docs/first_flow/first_flow.md) section of the guide explains what that sample does and walks you through it.

## Make docs or guide changes
I am building the 'guide' with a modified version of mdbook (PR pending merge by the mdbook team) that allows me to build a book with content in the root folder (like this README.md) included in the book content.

After a PR is merged, and the build of `master` succeeds, the guide is rebuilt and the resulting html is deployed to Github Pages, and can be viewed [here](http://andrewdavidmackenzie.github.io/flow/)

## License
MIT

## Project structure
The Project is structured into a number of sub-folders, many of which are also rust crates with their own 
`Cargo.toml` file. There is a "workspace manifest" `Cargo.toml` in the root that includes many of them, but not
all.

See the [Structure section](docs/developing/structure.md) of the Guide for more details.
