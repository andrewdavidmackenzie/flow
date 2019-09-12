# Provider
This crate implements a `content provider` that resolves URLs and then get's the content of 
the url for flowclib and flowrlib, keeping them independant of IO operations and able to be
compiled to WebAssaembly and used in other implementations (e.g. a Web browser or Electron app).