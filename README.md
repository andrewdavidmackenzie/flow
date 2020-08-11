[![Build Status](https://travis-ci.org/andrewdavidmackenzie/flow.svg?branch=master)](https://travis-ci.org/andrewdavidmackenzie/flow)
[![codecov.io](https://codecov.io/gh/andrewdavidmackenzie/flow/coverage.svg?branch=master)](https://codecov.io/gh/andrewdavidmackenzie/flow/branch/master)
# Welcome!
Welcome to my project called `flow` for defining and running parallel, [dataflow programs](https://en.wikipedia.org/wiki/Dataflow_programming) like this one:

![First flow](http://andrewdavidmackenzie.github.io/flow/samples/fibonacci/context.dot.svg)

It currently consists of libraries for compiling (`flowclib`) and running (`flowrlib`) flows as 
well as command line binaries (`flowc` and `flowr`) that use the libraries.

As data-flow programming fits very well with a visualization of
processes operating on data flowing from other processes, a visual programming method
would be great. But I've only touched the surface of an IDE, disapointed by the current state of
of cross-platform UI programming. That's potentially a great (but massive) project 
in itself.
 
In this project I work on the programming "semantics" as I implement some sample programs.
It's a journey of discovery of writing something like this (for me), learning 
rust in the process and learning how such a paradigm could work.
 
Learn more about what it is and why I created it by reading the [Introduction section](docs/introduction/introduction.md), 
or dive right in and see if you can understand [Your First Flow](docs/first_flow/first_flow.md) with zero previous knowledge and just your programmer's intuition.
 
This README.md forms part of the ['flow' Guide](http://andrewdavidmackenzie.github.io/flow/) published using gh-pages.

If you want to encourage me then you can ["patreonize me"](https://www.patreon.com/andrewmackenzie) :-)

## Install Pre-requisites and Build
You need [Git](https://git-scm.com) to clone the repo.

### Clone the repo
From your command line:

```bash
git clone https://github.com/andrewdavidmackenzie/flow.git
```

### Install build tools
You need `make` and a rust toolchain (rustup, cargo, rustc) to build from source (I suggest using [rustup](https://rustup.rs/))

Some crates need native (development) libraries to build, including `gtk3`, `ssl` etc.

Other build pre-requisites include `mdbook`, `mdbook-linkcheck` and `graphviz` to build the docs.

There is a `make` target to install them all for you that should work on `macos` and `linux` variants using `apt-get` 
or `yum` package managers (PRs to Makefile are welcome for other linux package managers).

After cloning the repo and in the `flow` directory, install the pre-requisites using:
```bash
make config
```

### Build and test
To build and test, including building and testing docs and running the samples and checking their output is correct:

```bash
make
```

## Run your first 'flow'
Run the 'first' sample flow using:

```cargo run -- samples/first```

You should get a series of numbers output to the terminal.

The [first flow](docs/first_flow/first_flow.md) section of the guide explains what that sample does and walks you through it.

## Make docs or guide changes
After a PR is merged, and the build of `master` succeeds, the guide is rebuilt and the resulting html is deployed for viewing [here](http://andrewdavidmackenzie.github.io/flow/)

## License
MIT

## Project structure
The Project is structured into a number of rust crates that form part of a rust cargo "workspace".
 
See the [Structure section](docs/developing/structure.md) of the Guide for more details.
