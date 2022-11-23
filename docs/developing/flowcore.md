# `flowcore`

See also: [Code docs](http://andrewdavidmackenzie.github.io/flow/code/doc/flowcore/index.html)

`flowcore` is a library of structs and traits related to `flow` that are shared between multiple
crates in the `flow`project.

## `Implementation` trait

This is a trait that implementations of flow 'functions' must implement in order for them to be invoked
by the flowrlib (or other) run-time library.

An example of a function implementing the `Implementation` trait can be found in the
docs for [Implementation](http://andrewdavidmackenzie.github.io/flow/code/doc/flowcore/trait.Implementation.html)

## `Provider`
This implements a `content provider` that resolves URLs and then gets the content of the url.

## Features
`flowcore` crate supports a number of "features" for conditional compiling with more or less features.

## features
These are the conditionally compiled features of `flowcore`:
- default - none are activated by default
- context - makes this crate aware of the flow context functions or not
- debugger - feature to add the debugger
- online_tests - run any tests activated by this feature
- meta_provider - include the meta provider for resolving "lib://" and "context://" Urls
- file_provider - include a provider to fetch content from the file system
- http_provider - include a provider to fetch content from the web

Examples
- `flowrlib` library crate compiles `flowcore` activating the "file_provider", "http_provider",
  "context" and "meta_provider" features
- `flowr` compiled `flowcore` activating the "context" feature as it provides `context functions`. It has a 
number of features that, if activated, active corresponding features in `flowcore` (`flowr` "debugger"
feature actives "flowcore/debugger" feature.) and it depends on `flowrlib` (above) that in turn activates
features
- `flowrex` compiles `flowcore` with the default set of features (which is the minimal set in the case
of `flowcore` as it does not provide ant `context functions` ("context" feature), nor does it coordinate flow
running and provide a debugger ("debugger" feature), nor does it have the need for running "online_tests",
and lastly it does not fetch content via any of the various "providers" ("meta_provider", "file_provider",
and "http_provider" features).