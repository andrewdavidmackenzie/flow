use serde_derive::{Deserialize, Serialize};

/// Types of `Params` used in communications between the debugger and the debug_client
#[derive(Serialize, Deserialize, PartialEq)]
pub enum Param {
    /// A "*" style parameter - meaning will depend on the `Command` it's use with
    Wildcard,
    /// A positive integer was specified - could be a function or a job number
    Numeric(usize),
    /// A descriptor for the `Output` of a `Function` was specified
    Output((usize, String)),
    /// A descriptor for the `Inout` of a `Function` was specified
    Input((usize, usize)),
    /// A description of a "block" (when one function is blocked from running by another) was specified
    Block((Option<usize>, Option<usize>)),
}