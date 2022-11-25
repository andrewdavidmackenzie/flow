# Running the flow

This flow exists as a sample in the `flowsamples/fibonacci` folder. See the 
[root.toml](../../flowsamples/fibonacci/root.toml) root flow definition file

You can run this flow and observe its output from the terminal, while in the flow project root folder:

```shell script
> cargo run -p flowc -- -C flowr/src/cli flowsamples/fibonacci
```

`flowc` will compile the flow definition from the root flow definition file (`root.toml`) using the `context functions`
offered by `flowr` (defined in the `flowr/src/cli` folder) to generate a `manifest.json` compiled flow manifest in the 
`flowsamples/fibonacci` folder.

`flowc` then runs `flowr` to execute the flow.

`flowr` is a Command Line flow runner and provides implementations for `context` functions to read and write to `stdio` (e.g. `stdout`).

The flow will produce a fibonacci series printed to Stdout on the terminal.

```shell script
> cargo run -p flowc -- -C flowr/src/cli flowsamples/fibonacci
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