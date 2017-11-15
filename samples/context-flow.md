# Context flow describes the SOURCES and SINKS of data, any "dangling" inputs/outputs to/from the main flow
# and identifies the main flow, which is describes in a separate file

# Name of this flow
name: Hello World

# Declare SOURCES it contains
sources:

# Declare SINKS it contains
sinks:
    # Each should have a name that it can be referred to by
    name: STDOUT
    # Each should identify a "driver" for the system that is used to flow data to it
    driver: system/STDOUT

# Flows can define values (constants) that it contains and that will flow somewhere
values:
    # They must be named to identify connections - could be a system generated name
    name: message
    # They should be types
    type: String
    # The literal value
    value: "Hello World!"
    
# List of sub-flows to include. At the context level only one can be included
flows:
    1.flow

# This is a list of connections of inputs and outputs to/from sub-flows to the parent flow
# This MUST match the list of inputs/outputs described in the child-flow
# there can be no 'dangling' inputs from a SOURCE
connections:
    # Example of naming a connection from a sub-flow
    name: 1.flow/message
    type: String
    destination: STDOUT
    
    # Example naming a connection from the list of values
    name: values/message
    type: String
    destination: STDOUT

    # describe any 'dangling' outputs of the flow here, without specifying what connected to
    # other_output:


