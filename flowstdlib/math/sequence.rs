/// Generate a sequence of numbers between a start and end number that is supplied
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "sequence"
/// source = "lib://flowstdlib/math/sequence"
/// ```
///
/// ## Inputs
/// * `start` - the first number of the sequence to generate, type `Number`
/// * `end` - the last number of the sequence, type `Number`
///
/// ## Outputs
/// * `sequence` the output sequence of type `Number`
/// * `done` a signal of value `true` that is output when the sequence ends, type `Bool`
pub struct Sequence;