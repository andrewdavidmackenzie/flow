## IO References
This uniquely identifies an IO from a Data source (flow/value/function).

If any flows or functions defined in other files are referenced with an alias, then it should be
used in the IO references to inputs or outputs of that referenced flow/function.

### Data sources
Sources of data which are valid for the *from* field of a connection are:

Inside a flow:
- `input/{input_name}`
- `value/{value_name}`

Referenced from a flow:  
- `process/{flow_alias|flow_name}/{output_name}`
- `process/{function_alias|function_name}/{output_name}`

### Data sinks
Sinks for data which are valid for the *to* field of a connection are:

Inside a flow:  
- `output/{output_name}`
- `value/{value_name}`

Referenced from a flow:  
- `process/{flow_alias|flow_name}/{input_name}`
- `process/{function_alias|function_name}/{input_name}`

TODO 

Using named outputs to destructure a JSON value or Array or Map