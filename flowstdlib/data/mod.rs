//! ## Control (//flowstdlib/data)
//! Some generic Processes that act on data.

/// A module that implements accumulating an array from a number of elements
pub mod accumulate;
/// A module that implements a data buffer
pub mod buffer;
/// A module that implements composing an array from a number of elements
pub mod compose_array;
/// A module that duplicates the rows in an array
pub mod duplicate_rows;
/// A module with a function to get info about a Value
pub mod info;
/// A module that removes elements from an array
pub mod remove;
/// A module with a function for transposing a Matrix
pub mod transpose;
/// A module that zips two sets of data into a set of tuples of data
pub mod zip;
