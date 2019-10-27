//! A Module with functions for controlling data flow
//! ## Control (//flowstdlib/control)
//! Functions to control the flow of data on connections between other processing functions

/// A module with functions to compare data elements
pub mod compare;

/// A module with functions for joining data
pub mod join;

/// A module with functions to control the flow of data based on comparisons
pub mod tap;

/// A module that is just a flow definition for passing a value depending on a control value
pub mod pass_if_lte;