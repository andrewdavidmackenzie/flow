# Installing flow on your system

There are three main options to getting a working install of flow on your system:
- from source
- downloading a release
- homebrew tap

## From Source
All pretty standard:
- clone this repo
- install pre-requisites with `make config`
- build and test with `make`

That will leave binaries (such as `flowc` and `flowrcli` etc) in target/debug, and `flowstdlib`
installed into $HOME/.flow/lib.

You can use them from there or you can install using `cargo install`.

You will need to configure the environment variable `FLOW_LIB_PATH` to be `$HOME/.flow/lib` to allow
the compiler to find libraries (`flowstdlib` for now).

## Downloading the latest release
From [latest GitHub release](https://github.com/andrewdavidmackenzie/flow/releases/latest) download
and manually install the executables for your target system:
- flowc
- flowrcli
- flowrex
- flowrgui

Then download the portable WASM `flowstdlib` and expand to the directory `$HOME/.flow/lib/flowstdlib`

You will need to configure the environment variable `FLOW_LIB_PATH` to be `$HOME/.flow/lib` to allow
the compiler to find libraries (`flowstdlib` for now).

## Homebrew tap
A homebrew tap repo is maintained [here](https://github.com/andrewdavidmackenzie/homebrew-dataflow) which 
you can use to install with homebrew:

```
> brew tap dataflow
```

That should install the binaries, the portable `flowstdlib` WASM library and be ready for running.