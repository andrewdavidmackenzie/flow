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
cd flow
# Build and run the app
make
```

## Packaging the app

You can package easily for the platform you are currently running on with:

```
make package
```

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

## License

MIT
