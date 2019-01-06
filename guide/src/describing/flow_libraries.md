## Flow Libraries
Functions and Flows can be defined as libraries and shared re-used by other flows.

The runtime includes some basic ones in a `flowstdlib` standard library.

Others can be shared by developers.

The references to flows or functions specify a source, that will be searched for in a defined
FLOWLOADPATH, that currently uses relative paths to the current flow's CWD.

If the specified name is not found there, the load path will be searched for it.

TODO

Format and examples of how to reference library functions from flows