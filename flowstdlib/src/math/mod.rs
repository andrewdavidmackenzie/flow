//! functions for maths operations on data
//! ## Math (//flowstdlib/math)
//! Math Functions

use serde_json::{json, Value};

/// Return a JSON integer when the float value is a whole number, otherwise a float.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::float_cmp
)]
pub(crate) fn numeric_json(f: f64) -> Value {
    if f.fract() == 0.0 && f.abs() < i64::MAX as f64 {
        let i = f as i64;
        if (i as f64) == f {
            return json!(i);
        }
    }
    json!(f)
}

/// A flow to generate numbers within a range
pub mod range;

/// A flow to generate a sequence of numbers
pub mod sequence;

/// A module with a function to add two `Numbers`
#[path = "add/add.rs"]
pub mod add;
/// A module with functions to compare data elements
#[path = "compare/compare.rs"]
pub mod compare;
/// A module with a function to calculate the cosine of a number (radians)
#[path = "cos/cos.rs"]
pub mod cos;
/// A module with a function to divide two `Numbers`
#[path = "divide/divide.rs"]
pub mod divide;
/// A module with a function to multiply two `Numbers`
#[path = "multiply/multiply.rs"]
pub mod multiply;
/// A module with a function to split a range of `Numbers`, into two sub-ranges
#[path = "range_split/range_split.rs"]
pub mod range_split;
/// A module with a function to calculate the sine of a number (radians)
#[path = "sin/sin.rs"]
pub mod sin;
/// A module with a function to calculate the square root of a `Number`
#[path = "sqrt/sqrt.rs"]
pub mod sqrt;
/// A module with a function to subtract two `Numbers`
#[path = "subtract/subtract.rs"]
pub mod subtract;
/// A module with a function to calculate the tangent of a number (radians)
#[path = "tan/tan.rs"]
pub mod tan;
