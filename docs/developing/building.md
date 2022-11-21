# Building `flow`

## Install Pre-requisites and Build
You need [Git](https://git-scm.com) to clone the repo.

### Clone the repo
From your command line:

`git clone https://github.com/andrewdavidmackenzie/flow.git`

### Install build tools
You need `make` and a rust toolchain (cargo, rustc, clippy) to build from source
(I suggest using [rustup](https://rustup.rs/)).

Once you have those, you can install the remaining pre-requisites using:

`make config`

These include libraries like `ssl` and tools like `mdbook`, `mdbook-linkcheck` and `graphviz` to build the docs.

The `make config` target should install them all for you. It should work on `macos` and `linux` variants using `apt-get`
or `yum` package managers (PRs to Makefile are welcome for other linux package managers).

### Build and test
To build and test, including building and testing docs and running the samples and checking their output is correct:

`make`

**NOTE**
The first time you build, it will take a long time. This is due to a large number of function implementations
in the `flowstdlib` standard library - each being compiled to WASM as individual projects. After the first build,
dependencies are tracked by the `flowc` compiler and implementations are only re-compiled when required.

## Make docs or guide changes
After a PR is merged, and the build of `master` succeeds, the guide is rebuilt and the resulting html is deployed for
viewing [here](http://andrewdavidmackenzie.github.io/flow/)

## Project components and structure
The Project is structured into a number of rust crates that form part of a rust cargo "workspace".

Currently, two binaries are built: `flowc` the flow compiler and `flowr` the flow runner.

See the [Project Components and Structure section](../introduction/structure.md) of the Guide for more details.

## Contributing
I organize all issues in a [Github Project](https://github.com/andrewdavidmackenzie/flow/projects/2)
and chose things to work on from the "Next" column. I have only marked a couple of issues with "help wanted" label
but I can do more if there is interest. If in doubt reach out to me by email, or GitHub issue.
