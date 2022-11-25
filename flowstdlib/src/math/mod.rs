//! functions for maths operations on data
//! ## Math (//flowstdlib/math)
//! Math Functions

/// A module with functions to compare data elements
#[path = "compare/compare.rs"]
pub mod compare;
/// A module with a function to add two `Numbers`
#[path = "add/add.rs"]
pub mod add;
/// A module with a function to divide two `Numbers`
#[path = "divide/divide.rs"]
pub mod divide;
/// A module with a function to multiply two `Numbers`
#[path = "multiply/multiply.rs"]
pub mod multiply;
/// A module with a function to split a range of `Numbers`, into two sub-ranges
#[path = "range_split/range_split.rs"]
pub mod range_split;
/// A module with a function to subtract two `Numbers`
#[path = "subtract/subtract.rs"]
pub mod subtract;
/// A module with a function to calculate the square root of a `Number`
#[path = "sqrt/sqrt.rs"]
pub mod sqrt;