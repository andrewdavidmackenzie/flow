# Flow IDE

See also: [Code docs](../code/doc/flowide/index.html)

This is an IDE for flow that uses WebAssembly compilation of flow libraries, plus web-sys.rs rust crate to 
interact with the dom from rust, plus wasm-bindgen to generate bindings. Packaged as an electron app.

# Build and run web version
To build and run just type ```make web```

# Build and run electron version
To build and run just type ```make app```

Other build targets include:
- `clean` - clean all compiler output (entire target directory) and generated files