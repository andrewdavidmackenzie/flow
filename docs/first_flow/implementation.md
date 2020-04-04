# Real Implementation

This flow exists as a sample in the `samples/first` folder and is written to be as simple as possible,
not using nested flows or similar.

### Running the corresponding sample
You can run this first flow and observe its output from the terminal, while in the project root folder:

```shell script
> cargo run -- samples/first
```

This should generate the `manifest.json` manifest for the flow and then run it using `flowr`.
`flowr` is a flow runtime and as such supplies implementations for all the `flowruntime` functions (e.g. `stdout`).

The flow produces a fibonacci series:
`
> cargo run -- samples/first
   Compiling flowstdlib v0.6.0 (/Users/andrew/workspace/flow/flowstdlib)
    Finished dev [unoptimized + debuginfo] target(s) in 1.75s
     Running `target/debug/flowc samples/first`
1
1
2
3
5
8
...... lines deleted ......
2880067194370816120
4660046610375530309
7540113804746346429

>