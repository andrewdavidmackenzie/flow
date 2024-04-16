## Multiply (//flowstdlib/matrix/multiply)
Multiply two matrices.

This flow is designed to stress particular aspects of the runtime:

* deserialization of an array of objects (a matrix or array/array/numbers in this case) into a lower order 
structure (array/number in this case, which are the rows and columns of the matrices that are fed to `multiply_row`). 
* The send of one value (a matrix with its rows repeated) from a previous function the matrix is deserialized and 
produces many writes of many values (rows) in one "tick", thus piling up multiple values at the destination function's
inputs
* When taking those values from the function to create new jobs on the ready queue, the runtime attempts to maximize
parallelization and creates as many jobs as inputs sets of values it can take.
* When dispatching new jobs for execution, taking those jobs from the ready job queue, the runtime again tries to
maximize parallelization and creates many jobs for the same function at once. Those jobs are dispatches and start
executing in parallel (how many and in which order depends on the maximum number of parallel jobs allowed, if the limit
is set, the number of cores and hence job executors being used, and previous jobs completing on those executors and
their input queues).
* So, the order of jobs dispatched will match the order of the elements of the original structure that was deserialized.
* But the order of _completion_ of jobs in not guaranteed, and they can arrive out of order.
* When constructing the final matrix that is the multiplication of the two input matrices, the order of elements
in rows, and rows in the matrix is critical.
* Thus the matrix multiplication algorithm here attaches row and column indexes as part of the values, and they proceed
thru the `flowstdlib` matrix functions to preserve and combine them into (row,column) pairs.
* These pairs are used at the end of the algorithm by `compose_matrix` to write the elements calculated into the 
correct (row, column) positions in the matrix, giving the correct result.

Writing algorithms like this, that require strict preservation of order in some parts, while desiring to maximize 
parallelization of execution, require that extra work and are a bit of a pain to do. We will look into ways 
that the language and runtime can help make this easier in the past, without breaking the maxim that
"things can happen out of order" and programmers should not rely on any inherent order of things happening that
is not determined by the data dependencies expressed in the graph.

### Include using
```toml
[[process]]
source = "lib://flowstdlib/matrix/multiply"
```
### Flow Graph
<a href="sequence.dot.svg" target="_blank"><img src="sequence.dot.svg"></a>

Click image to navigate flow hierarchy.