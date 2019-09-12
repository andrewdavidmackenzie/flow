# ide-native
This is an experimental crate that instead of using WebAssembly, Javascript and Electron
to build a "native" app, uses ffi to call rust functions directly.

It's still unclear what's the best/easiest way to build a native app that uses the `flowclib` and `flowrlib` 
rust libraries.