//! Runtime library for flow execution. This will be linked with code generated from a flow definition
//! to enable it to be compiled and ran as a native program.
pub mod info;
pub mod execution;
pub mod value;
pub mod implementation;
pub mod function;
pub mod runnable;
pub mod zero_fifo;