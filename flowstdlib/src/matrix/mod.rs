//! functions for matrix operations
//! ## Data (//flowstdlib/matrix)

/// A flow to multiply two matrices
pub mod multiply;

/// A Module that duplicates rows in a matrix
#[path = "duplicate_rows/duplicate_rows.rs"]
pub mod duplicate_rows;

/// A module that does matrix row multiplication
#[path = "multiply_row/multiply_row.rs"]
pub mod multiply_row;

/// A module with a function for transposing a Matrix
#[path = "transpose/transpose.rs"]
pub mod transpose;

/// A module with a function to compose a Matrix from elements
#[path = "compose_matrix/compose_matrix.rs"]
#[allow(clippy::module_name_repetitions)]
pub mod compose_matrix;