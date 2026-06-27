# Developing flow

## Supported Platforms
CI tests run on macOS and Linux (x86_64 and aarch64). Other platforms may work
since Rust projects are portable, but these are the ones actively tested.

## Pre-requisites
To build and test flow you need:
- Rust toolchain (`rustup`, `cargo`, `rustc`) with the `wasm32-unknown-unknown` target
- `clippy` for lint checks
- `zmq` (Zero Message Queue) library
- `mdbook` and `mdbook-linkcheck` for building the book

## Getting Started

### Clone the repo
```bash
git clone https://github.com/andrewdavidmackenzie/flow.git
```

### Install build tools
You need `make` and a Rust toolchain (I suggest [rustup](https://rustup.rs/)).
Once you have those, install the remaining dependencies with:

```bash
make config
```

This installs the wasm target, clippy, mdbook, and other tools.
It works on macOS and Linux variants using `apt-get` or `yum`.

### Build and test
```bash
make
```

This builds everything, runs all tests (including examples), and builds the book.
The first build takes a while due to WASM compilation of flowstdlib functions.
Subsequent builds are incremental.

## Make Targets
- `make` — full build, test, and book (default)
- `make build` — build libs and binaries only
- `make clippy` — run clippy on all code including tests
- `make test` — run all tests, including example output checks
- `make book` — build the book and check links

## Project Structure
The project is split into several Rust crates in a cargo workspace:
- Proc macros need their own crate
- Core types and code are shared across compiler and runner crates
- Libraries are separated from CLI binaries to enable UI applications
- The standard library (`flowstdlib`) is separate from the compiler and runtime
- Feature flags allow optional compilation of debugger, metrics, etc.

The main binaries are `flowc` (compiler), `flowrcli` (CLI runner), and
`flowrgui` (GUI runner). See [Project Structure](../introduction/structure.md)
for details.
