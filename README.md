[![Build Status](https://travis-ci.org/andrewdavidmackenzie/flow.svg?branch=master)](https://travis-ci.org/andrewdavidmackenzie/flow)

# Flow

Flow is a library, cli and app for the visual creation and execution of asynchronous, data-driven, programs. See [Flow Programming] section below.

At the moment the app is just a skeleton Electron app while I play with it and learn Electron, and integrating library code written in rust and compiled to WebAssembly.

## Pre-requisites

You need [Git](https://git-scm.com) and [Node.js](https://nodejs.org/en/download/) (which comes with [npm](http://npmjs.com)) installed.

The project uses the electron-forge packaging tool which you can install with:
```
npm install -g electron-forge
```

See https://github.com/electron-userland/electron-forge for more details on how to use electron-forge.

## To Run the app

With pre-requisites installed, from your command line:

```bash
# Clone this repository
git clone https://github.com/andrewdavidmackenzie/flow.git
# Go into the repository directory
cd flowui
# Build and run the app
make
```

Note: If you're using Linux Bash for Windows, [see this guide](https://www.howtogeek.com/261575/how-to-run-graphical-linux-desktop-applications-from-windows-10s-bash-shell/) or use `node` from the command prompt.

## Packaging the app

You can package easily for the platform you are currently running on with:

```
make package
```

## Running the command line tools
There are currently two command line tools 'run' and 'check'.

They can be built and run using cargo:
```
cargo run --bin run
```

```
cargo run --bin check
```

## Travis Locally

If you have travis-CI problems, and (like me) get tired of pushing changes to try and figure it out, you can run a travis-node-js Docker Image locally, log in to it and try and figure it out, thus:

- Download and install the Docker Engine.
- Select an image from Quay.io. If you're not using a language-specific image pick travis-ruby. Open a terminal and start an interactive Docker session using the image URL:
- docker run -it quay.io/travisci/travis-ruby /bin/bash
- Switch to the travis user:
- su - travis
- Clone your git repository into the current folder (/home/travis) of the image.
- Go into the 'flowui' directory
- Manually install any dependencies.
- Manually run your Travis CI build command.


Flow Programming
##

This is an exploration project of some ideas I have for programming using a description of data flows and transformations.

I'd like a visual app to create the app descriptions (producing a program description in a set of yaml files at the moment) and also be able to run them, as well as CLI tools to validate and run such a program.

I plan to try and develop a few trivial, and then simple, and later maybe more complicated sample programs to tease out the initial complexities.

The running of the algorithms would just be to demonstrate that the method can be used to describe the programs tried, and will not be performant, or anything like a compiled solution that would be required in a final solution.

Flow Descriptions
###

Flows may have zero or more inputs, outputs, values (constants), functions, and other sub-flows.
flow = [input] + [output] + [flow] + [values] + [functions]

To Consider
###
Error handling

Logging
###
Using the 'log' framework in libraries and main binary code to log.
https://doc.rust-lang.org/log/log/index.html

Using the log4rs log implementation, see https://github.com/sfackler/log4rs,
configured in each of the binaries's main() function.

log.toml is the log configuration file used by log4rs.

## License

MIT
