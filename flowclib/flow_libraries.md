Functions and FLows can be defined as libraries and shared re-used by other flows.

The runtime will include some basic ones in a flowstdlib standard library.

Others can be shared by developers.

The references to flows or functions specify a source, that will be searched for in a defined
LOAD_PATH, that currently uses relative paths to the current flow's CWD.

If the specified name is not found there, the load path will be searched for it.

