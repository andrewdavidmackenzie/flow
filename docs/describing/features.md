## Complete Feature List
This is a complete list of features implemented in the description of flows:

* Root Flow
    * Cannot have any outputs from this flow as there is no parent level
* All Flows (Root and Children)
    * Can contain elements directly inside the root flow description
    * Child flow inclusion from description in its own flow file in current project or different project
    * Named outputs from child flow, referenced by parent flow for connections
* Functions
    * With just inputs
    * With just outputs
    * With inputs and outputs
    * Use of aliases to refer to functions with different names inside a flow
* Use of Library Functions
* Providing a Custom function (in rust) with a flow
* Destructuring of output value into multiple named outputs
* Connections between outputs and inputs
    * Connections between inputs and outputs of functions, values, current flow and sub-flows
    * Multiple connections to a single inputs (first arrived wins)
    * Multiple connections from a single output (value is copied to all destinations)
    * Connections to values don't require input name as only have one input
    * Connections from values don't require output name as only have one output
    * Functions with single input can have a connection to it without naming the input
    * Functions with single output can have a connection from it without naming the output
* Libraries of functions can be built and described, like flowstdlib, and referenced in flows
* Run-time functions for
    * Retrieving arguments from the flow's invocation
    * STDIN/STDOUT/STDERR
    * Retrieving the value of Environment Variables
