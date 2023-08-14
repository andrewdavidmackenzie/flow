# Running the flow

This flow exists as an example in the `flowr/examples/fibonacci` folder. See the 
[root.toml](../../flowr/examples/fibonacci/root.toml) root flow definition file

You can run this flow and observe its output from the terminal, while in the flow project root folder:

```shell script
> cargo run -p flowc -- -C flowr/src/bin/flowrcli flowr/examples/fibonacci
```

`flowc` will compile the flow definition from the root flow definition file (`root.toml`) using the `context functions`
offered by `flowrcli` (defined in the `flowr/src/bin/flowrcli/context` folder) to generate a `manifest.json` compiled flow manifest in the 
`flowr/examples/fibonacci` folder.

`flowc` then runs `flowrcli` to execute the flow.

`flowrcli` is a Command Line flow runner and provides implementations for `context` functions to read and write to `stdio` (e.g. `stdout`).

The flow will produce a fibonacci series printed to Stdout on the terminal.

```shell script
> cargo run -p flowc -- -C flowr/src/bin/flowrcli flowr/examples/fibonacci
   Compiling flowstdlib v0.6.0 (/Users/andrew/workspace/flow/flowstdlib)
    Finished dev [unoptimized + debuginfo] target(s) in 1.75s
     Running `target/debug/flowc flowr/examples/first`
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