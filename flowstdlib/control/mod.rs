//! A Module with functions for controlling data flow
//! ## Control (//flowstdlib/control)
//! Functions to control the flow of data on connections between other processing functions

/// A module with functions to compare data elements and output a value depending on comparison
pub mod compare_switch;

/// A module with functions for joining data
pub mod join;

/// A module with functions to control the flow of data based on a control value
pub mod tap;

/// A module with functions to route data based on a control value
pub mod route;

/// A module with functions to select data on output on a control value
pub mod select;
