Flowrex
==

A minimal executor binary for flow job execution. It uses *just* the "p2p" content provider to get the content 
of wasm files for implementations from the p2p network it is participating in. 

For that reason it compiles `flowrlib` with `default-features = true` and then re-enables the `p2p_provider` with a
feature. As features are additive across crates in a workspace, this crate is *not* in the overall `flow` workspace
and has it-s own `target` directory where `flowrlib` will be built with other features disabled.