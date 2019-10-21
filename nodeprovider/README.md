# `nodeprovider`
This crate implements a node version of a `content provider` that resolves URLs and 
then get's the content of the url for flowclib and flowrlib.
 
This keeps them independant of IO operations and able to be compiled to WebAssaembly 
and used in other implementations (e.g. an Electron app).

Electron includes NodeJS and so it is used to interact with FileSystem, etc.

Under development.