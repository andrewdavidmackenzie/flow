## Step-by-Step
Here we walk you through the execution of the previous "my first flow" (the fibonacci series sample).

Compiled flows consist of only functions, so flow execution consists of executing functions, or more precisely, jobs
formed from a set of inputs, and a reference to the function implementation.

### Init
The flow manifest (which contains a list of Functions and their output connections) is loaded.

Any function input that has an input initializer on it, is initialized with the value provided in the initializer.

Any function that has either no inputs (only `context funcitons` are allowed to have no inputs, such as `Stdin`) or
has a value on all of its inputs, is set to the ready state.

### Execution Loop
The next function that is in the ready state (has all its input values available, and is not blocked from sending 
its output by other functions) has a job created from its input values and the job is dispatched to be run.

Executors wait for jobs to run, run them and then return the result, that may or may not contain an output value.

Any output value is sent to all functions connected to the output of the function that the job ran for. 
Sending an input value to a function may make that function ready to run.

The above is repeated until there are no more functions in the ready state, then execution has terminated and the flow ends.

### Specific Sequence for this example
Below is a description of what happens in the flor runtime to execute the flow.

You can see log output (printed to STDOUT and mixed with the number series output) of what is happening using 
the `-v, verbosity <Verbosity Level>` command line option to `flowr`. 
- Values accepted (from less to more output verbosity) are: `error` (the default), `warn`, `info` `debug` and `trace`.

#### Init:
* The "i2" input of the "add" function is initialized with the value 1
* The "ii" input of the "add" function is initialized with the value 0
* The "add" function has a value on all of its inputs, so it is set to the ready state
* STDOUT does not have an input value available so it is not "ready"

#### Loop Starts
Ready = ["add"]

- "add" runs with Inputs = (0, 1) and produces output 1
  - value 1 from output of "add" is sent to input "i2" of "add"
    - "add" only has a value on one input, so is NOT ready
  - value 1 from output of "add" is sent to default (only) input of "Stdout"
    - "Stdout" has a value on all of its (one) inputs and so is marked "ready"
  - input value "i2" (1) of the executed job is sent to input "i1" of "add"
    - "add" now has a value on both its inputs and is marked "ready"

Ready = ["Stdout", "add"]

- "Stdout" runs with Inputs = (1) and produces no output
    - "Stdout" converts the `number` value to a `String` and prints "1" on the STDOUT of the terminal
    - "Stdout" no longer has values on its inputs and is set to not ready

Ready = ["add"]

- "add" runs with Inputs = (1, 1) and produces output 2
  - value 2 from output of "add" is sent to input "i2" of "add"
    - "add" only has a value on one input, so is NOT ready
  - value 2 from output of "add" is sent to default (only) input of "Stdout"
    - "Stdout" has a value on all of its (one) inputs and so is marked "ready"
  - input value "i2" (1) of the executed job is sent to input "i1" of "add"
    - "add" now has a value on both its inputs and is marked "ready"

Ready = ["Stdout", "add"]

- "Stdout" runs with Inputs = (2) and produces no output
  - "Stdout" converts the `number` value to a `String` and prints "2" on the STDOUT of the terminal
  - "Stdout" no longer has values on its inputs and is set to not ready

Ready = ["add"]

The above sequence proceeds, until eventually:

- `add` function detects a numeric overflow in the add operation and outputs no value.
  - No value is fed back to the "i1" input of add 
    - "add" only has a value on one input, so is NOT ready
  - No value is sent to the input of "Stdout"
    - "Stdout" no longer has values on its inputs and is set to not ready

Ready = []

No function is ready to run, so flow execution ends.

Resulting in a fibonacci series being output to Stdout
```
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
