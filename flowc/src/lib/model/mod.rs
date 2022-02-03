/// `connection` defines the connection between one process output to another process's input
pub mod connection;
/// `datatype` specifies the type o fdata permitted on a input, output or connection
pub mod datatype;
/// `flow` is the definition of an entire flow, including children flows
pub mod flow_definition;
/// `function` defines a function in a flow or library
pub mod function_definition;
/// `io` is the object used to define a process's inputs or outputs
pub mod io;
/// `name` is used to name various objects in the flow model
pub mod name;
/// `process` is a generic definition of a `function` or a `flow` so a flow refering to it or using
/// it does not need to know or define how it is implemented
pub mod process;
/// `process_reference` is an object used within a flow to reference a process defined elsewhere
pub mod process_reference;
/// `route` defines a location in the hierarchy of a flow and can locate a flow, a function, or one of
/// its inputs or outputs
pub mod route;
/// A custom deserializer for a String or a Sequence of Strings for Routes
mod route_array_serde;
/// A custom deserializer for a String or a Sequence of Strings for DataTypes
mod datatype_array_serde;
