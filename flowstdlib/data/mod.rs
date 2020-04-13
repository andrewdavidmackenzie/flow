//! ## Control (//flowstdlib/data)
//! Some generic Processes that act on data.

/// A module that implements accumulating an array from a number of elements
#[path = "accumulate/accumulate.rs"]
pub mod accumulate;
/// A module that implements a data buffer
#[path = "buffer/buffer.rs"]
pub mod buffer;
/// A module that implements composing an array from a number of elements
#[path = "compose_array/compose_array.rs"]
pub mod compose_array;
/// A module that duplicates objects into an array of them
#[path = "duplicate/duplicate.rs"]
pub mod duplicate;
/// A module that duplicates the rows in an array
#[path = "duplicate_rows/duplicate_rows.rs"]
pub mod duplicate_rows;
/// A module with a function to get info about a Value
#[path = "info/info.rs"]
pub mod info;
/// A module that does matrix row multiplication
#[path = "multiply_row/multiply_row.rs"]
pub mod multiply_row;
/// A module that removes elements from an array
#[path = "remove/remove.rs"]
pub mod remove;
/// A module that splits a String into an Array of Strings
#[path = "split/split.rs"]
pub mod split;
/// A module with a function for transposing a Matrix
#[path = "transpose/transpose.rs"]
pub mod transpose;
/// A module that zips two sets of data into a set of tuples of data
#[path = "zip/zip.rs"]
pub mod zip;
