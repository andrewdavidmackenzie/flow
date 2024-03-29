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

These include libraries like `ssl` and tools like `mdbook`, `mdbook-linkcheck` and `graphviz` to build the book.

The `make config` target should install them all for you. It should work on `macos` and `linux` variants using `apt-get`
or `yum` package managers (PRs to Makefile are welcome for other linux package managers).

### Build and test
To build and test (including running the examples and checking their output is correct) as well as building the book 
and ensuring all links are value, use:

`make`

**NOTE**
The first time you build (using `make` or `make all`), it will take a long time. 
This is due the function implementations in the `flowstdlib` standard library being compiled to WASM. 
After the first build, dependencies are tracked by the `flowc` compiler and implementations are only re-compiled when required.

## Make book changes
The book is rebuilt as part of every PR and Merge to master to ensure it is not broken.
The book is rebuild and deployed [here](http://andrewdavidmackenzie.github.io/flow/) on every release.

## Project components and structure
The Project is structured into a number of rust crates that form part of a rust cargo "workspace".

Currently, two binaries are built: `flowc` the flow compiler two flow runners `flowrcli` and `flowrgui`.

See the [Project Components and Structure section](../introduction/structure.md) of the book for more details.

## Contributing
I organize all issues in a [Github Project](https://github.com/andrewdavidmackenzie/flow/projects/2)
and chose things to work on from the "Next" column. I have only marked a couple of issues with "help wanted" label
but I can do more if there is interest. If in doubt reach out to me by email, or GitHub issue.
