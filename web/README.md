# Web version
This is a web application using flow (initially almost empty) that uses WebAssembly compilation of flow 
libraries, plus web-sys.rs rust crate to interact with the dom from rust, plus wasm-bindgen to generate bindings.

# Build and run
To build and run just type ```make run```

Build targets include:
- `build` (default) - just build from source
- `run` - depends on `build` which is done first, then starts npm web server to serve project from http://localhost:8080 
- `clean` - clean all compiler output (entire target directory) and generated files