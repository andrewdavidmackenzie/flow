# Real Implementation

This flow exists as a sample in the `samples/first` folder and is written to be as simple as possible,
not using nested flows or similar.

### Running the corresponding sample
You can run this first flow and observe its output from the terminal, while in the project root folder:

```
> cargo run -- samples/first
```

This should generate the flow and then run it with the runtime, producing a fibonacci series and eventually an
integer overflow panic when the next number becomes too big to hold in a rust integer, similar to this:

```
> cargo run -- samples/first
   Compiling flowstdlib v0.6.0 (/Users/andrew/workspace/flow/flowstdlib)
    Finished dev [unoptimized + debuginfo] target(s) in 1.75s
     Running `target/debug/flowc samples/first`
"1"
"1"
"2"
"3"
"5"
"8"
...... lines deleted ......
"4660046610375530309"
ERROR	- panic occurred in file '/Users/andrew/workspace/flow/flowstdlib/src/math/add.rs' at line 22
ERROR	- Process STDERR:
    Finished dev [unoptimized + debuginfo] target(s) in 0.12s
     Running `samples/first/rust/target/debug/context`

ERROR	- Exited with status code: 101
```

