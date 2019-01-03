## Supported OS
The CI tests for 'flow' run on Mac OS X and Linux. Others may well work as rust projects are pretty 
portable but I develop on Mac OS X and don't know the status on other OS.

## Pre-requisites required to build and test
These are the pre-requisites that are required to build and test 'flow':
* rust toolchain (rustup, cargo, rustc via rustup, etc )
   * nightly (still, due to wasm-bindgen), and stable
   * wasm32-unknown-unknown target for building wasm

For generating JS to wasm bindings:
	```wasm-bindgen-cli```

For building the guide:
	```mdbook``` and its ```mdbook-linkcheck```

These are only required if you want to publish the guide locally:
	sudo pip install --upgrade pip
	sudo pip install ghp-import
	
## Installing pre-requisites
You have to install pip, rustup, cargo and rust native toolchain yourself, I decided to stop 
short of futzing with people's installed compilers via scripts and Makefile targets.

There is a Makefile target 'config' that will attempt to install remaining dependencies most 
rust developers might not already have.
- ```make config```

That will add the wasm32-unknown-unknown target, install wasm-bindgen-cli, mdbook and mdbook-linkcheck 
using cargo and then upgrade pip and install ghp-import for publishing to Github Pages.

## Important make targets
- (default) ```make``` will build, run local tests and generate docs.
- ```make build-guide``` will just generate the HTML for the guide if you are writing docs. But better
would be to just ```cd guide && mdbook serve``` as that will track and update the generated content as 
you make changes, allowing you to view them instantly with a browser refresh.
- ```make test``` this should be what you run to check changes you have made work OK. At the moment it is the 
same as 'make travis' until I re-instate some tests I was having issues with.

## Contributing
There are many ways of contributing
- adding an issue with a bug report of an enhancement request or new feature idea
- adding to or correcting the docs and this guide
- adding a new sample
- improvements to the libraries, compiler, standard library, runtime
- improvements to unit or integration tests
- improvements to build processes (e.g. getting coverage reports working etc)

### Issues
Issues can be found in the [repo](https://github.com/andrewdavidmackenzie/flow/issues), if you are not yet a 
project contributor then just add a comment to one to say you'd like to work on it and I will avoid doing
the same. 

Adding new issues you find can also be helpful, although with my limited time on the project, fixing issues
and sending PRs are more welcome! :-)

### PRs
If you want to contribute a code or test or docs or tolling change....
- if no existing issue exists for it, create one so we can agree on what to do before starting (a good idea 
to make later PR merges easier to accept I think!)
- if an issue exists alread add a comment to it so I know you want to work on it
- fork the repo
- create a branch for the issue in your repo
- make your changes and update tests, docs and samples as required
- run tests ('make travis') before pushing to your branch
- wait for Travis to pass
- submit the PR, referencing the issue is a good ideas

### Testing a failing sample
If you have made a change (in source of compiler, or a sample definition) that is causing that sample to fail,
then you can easily run just the compile and test of that sample using a make target such as:
- 'make samples/fibonacci/test_output.txt'

where 'finonacci' is the name of the sample you want to test.

That make target depends on 'compiler' so it will make sure to recompile 'flowc' and any dependencies before it 
compiles, generates, builds the sample in question. It then runs the sample with pre-defined inputs and captures 
the output and compares it to previously generated "correct" output - passing if they are the same and failing if not.

### CI tests
The CI build and test run in Travis on each push to a branch or PR can be run locally using 'make travis'.
These tests include unit and integration tests, rust doc-tests and it also compiles, generates, runs and checks the 
output of all the samples found in the ./samples folder.

I recommend making sure this runs fine and passes before pushing to GitHub.

## Other less important make targets
- 'make package' this will prepare 'flowc' for publishing to crates.io, but this shouldn't be needed by 
most people. It should also package the electron app (when I fix that!).

- 'make clean' this will clean all generated files that 'cargo' knows about as well as the generated code and test 
output files for each of the samples, and should leave everything clean.