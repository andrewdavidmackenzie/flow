//! `model` module defines a number of core data structures that are used across the compiler
//! and the runtime and macros.

/// Definition of `RuntimeFunction` structure
pub mod runtime_function;

/// `connection` defines the connection between one process output to another process's input
pub mod connection;
/// `datatype` specifies the type of data permitted on a input, output or connection
pub mod datatype;
/// `flow` is the definition of an entire flow, including children flows
pub mod flow_definition;
/// `flow_manifest` is the struct that specifies the manifest of functions in a flow
pub mod flow_manifest;
/// `function` defines a function in a flow or library
pub mod function_definition;
/// `input` defines the struct for inputs to functions in a flow
pub mod input;
/// `io` is the object used to define a process's inputs or outputs
pub mod io;
/// `lib_manifest` defines the structs for specifying a Library's manifest and methods to load it
pub mod lib_manifest;
/// `metadata` defined structs for flow meta data
pub mod metadata;
/// `metrics` defines a struct for runtime execution metrics
pub mod metrics;
/// `name` is used to name various objects in the flow model
pub mod name;
/// `output_connection` defines a struct for a function's output connection
pub mod output_connection;
/// `process` is a generic definition of a `function` or a `flow` so a flow referring to it or using
/// it does not need to know or define how it is implemented
pub mod process;
/// `process_reference` is an object used within a flow to reference a process defined elsewhere
pub mod process_reference;
/// `route` defines a location in the hierarchy of a flow and can locate a flow, a function, or one of
/// its inputs or outputs
pub mod route;
/// `submission`defines a struct for submitting flows for execution
pub mod submission;
/// Traits used for the validation of Model structs
pub mod validation;
