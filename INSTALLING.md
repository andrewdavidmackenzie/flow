# Installing flow on your system

There are three main options to getting a working install of flow on your system:
- from source
- downloading the latest release
- using the install script

## From Source
All pretty standard:
- clone this repo (`git clone https://github.com/andrewdavidmackenzie/flow`)
- install pre-requisites by running `make config`
- build and test by running `make`

That will leave binaries (such as `flowc` and `flowrcli` etc) in `target/debug`, and `flowstdlib`
installed into `$HOME/.flow/lib`.

You can run the binaries from the target folder (make sure it is in your `$PATH`)
or you can install using

`cargo install`.

## Downloading the latest release
From [latest GitHub release](https://github.com/andrewdavidmackenzie/flow/releases/latest) download and manually install the executables for your target system:
- `flowc`
- `flowrcli`
- `flowrex`
- `flowrgui`

Then download the portable WASM `flowstdlib` and expand to the directory `$HOME/.flow/lib/flowstdlib`

## Install script
Install the latest release using the following install script from the source repo:

```
curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/install.sh | sh
```

That will:
* install the binaries:
   * `flowc`
   * `flowrcli`
   * `flowrex`
   * `flowrgui`
- install the portable `flowstdlib` WASM library into `$HOME/.flow/lib`
- install the context definition of `flowrcli` flow runner into `$HOME/.flow/runner`
- install the context definition of `flowrgui` flow runner into `$HOME/.flow/runner`

and leave things ready for running.

## Test your install
You can test your install works by running an example flow directly from the web, without cloning the repo to get
the examples:

```
> flowc -r flowrcli https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/flowr/examples/fibonacci/root.
toml
```

Which should download and run the fibonacci series example, which will produce a stream of numbers on standard output.