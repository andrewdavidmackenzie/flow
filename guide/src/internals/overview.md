## Internals Overview
In this section we provide some more details on what flowc does when you use it to compile, generate, build and run a flow.

The process includes these areas described in more detail in the following pages:
* Flow loading: the process of reading in the flow description and building an internal representation of it
* Flow compiling: take hierarchical flows representation loaded from previous stage and "compile down" to one more 
suited for project for flow project generation for execution.
* Flow execution: The generated project is loaded by the generic runtime library (flowrlib) and the functions are executed in turn.
