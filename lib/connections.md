# Connections

Connections connect a source of data to a sink of data of the same datatype, and are described using the 
"connections" table.

## Data sources
Sources of data which are valid for the *from* field of a connection are:

Inside a flow:  
- input/input_name
- value/value_name

Referenced from a flow:  
- flow/flow_name/output_name
- function/function_name/output_name

## Data sinks
Sinks for data which are valid for the *to* field of a connection are:

Inside a flow:  
- output/output_name
- value/value_name

Referenced from a flow:  
- flow/flow_name/input_name
- function/function_name/input_name

## Data Types
for a connection to be valid and used in execution of a flow, the data source must be found,
the data sink must be found and the two must be of matching DataTypes.

If those conditions are not met, then a connection will be dropped (with an error message output)
and the flow will attempted to be built and executed without it.