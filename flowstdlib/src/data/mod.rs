//! functions for generic operations on data
//! ## Data (//flowstdlib/data)

/// A module that implements accumulating an array from a number of elements
#[path = "accumulate/accumulate.rs"]
pub mod accumulate;
/// A module that implements String concatenation
#[path = "append/append.rs"]
pub mod append;
/// A module to get a value from an array at a runtime index
#[path = "array_get/array_get.rs"]
pub mod array_get;
/// A module to set a value in an array at a runtime index
#[path = "array_set/array_set.rs"]
pub mod array_set;
/// A module for computing stream avg of a stream
#[path = "avg/avg.rs"]
pub mod avg;
/// A module that counts occurrences of each value using bins
#[path = "bin_count/bin_count.rs"]
pub mod bin_count;
/// A module that counts data passed thru it
#[path = "count/count.rs"]
pub mod count;
/// A module that duplicates objects into an array of them
#[path = "duplicate/duplicate.rs"]
pub mod duplicate;
/// A module that enumerates entries of an array
#[path = "enumerate/enumerate.rs"]
pub mod enumerate;
/// A module with a function to get info about a Value
#[path = "info/info.rs"]
pub mod info;
/// A module for computing stream max of a stream
#[path = "max/max.rs"]
pub mod max;
/// A module for computing stream min of a stream
#[path = "min/min.rs"]
pub mod min;
/// A module that splits a String into an array of strings
#[path = "ordered_split/ordered_split.rs"]
pub mod ordered_split;
/// A module that removes elements from an array
#[path = "remove/remove.rs"]
pub mod remove;
/// A module with a function to sort values into an ordered array of numbers
#[path = "sort/sort.rs"]
pub mod sort;
/// A module that splits a String into an array of strings
#[path = "split/split.rs"]
pub mod split;
/// A module that zips two sets of data into a set of tuples of data
#[path = "zip/zip.rs"]
pub mod zip;
