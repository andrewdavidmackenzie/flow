//! functions for controlling data flow
//! ## Control (//flowstdlib/control)
//! Functions to control the flow of data on connections between other processing functions

/// A module with functions to compare data elements and output a value depending on comparison
#[path = "compare_switch/compare_switch.rs"]
pub mod compare_switch;

/// A function for joining data
#[path = "join/join.rs"]
pub mod join;

/// A function to control the flow of data based on a control value
#[path = "tap/tap.rs"]
pub mod tap;

/// A function to route data based on a control value
#[path = "route/route.rs"]
pub mod route;

/// A function to select data on output on a control value
#[path = "select/select.rs"]
pub mod select;

/// A function to select a value to pass based on index
#[path = "index/index.rs"]
pub mod index;
