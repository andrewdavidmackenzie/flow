# Real Implementation

This flow exists as a sample in the `samples/fibonacci` folder and is written to be as simple as possible,
not using nested flows or similar.

### Running the corresponding sample
You can run this first flow and observe its output from the terminal, while in the project root folder:

```shell script
> cargo run -- flowsamples/fibonacci
```

`flowc` will compile the flow definition (`root.toml`) and generate the `manifest.json` manifest which is 
then run using `flowr`.
`flowr` is a Command Line flow runner and provides implementations for `context` functions to read and write to `stdio` (e.g. `stdout`).

The flow produces a fibonacci series:

```shell script
> cargo run -p flowc -- flowsamples/fibonacci
   Compiling flowstdlib v0.6.0 (/Users/andrew/workspace/flow/flowstdlib)
    Finished dev [unoptimized + debuginfo] target(s) in 1.75s
     Running `target/debug/flowc flowsamples/first`
1
2
3
5
8
...... lines deleted ......
2880067194370816120
4660046610375530309
7540113804746346429
```