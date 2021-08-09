//! deserializer modules provides a number of deserializers from different formats and
//! also help methods to get a deserializer based on the file extension of a file referred to
//!by a Url

/// The JSON deserializer
pub mod json_deserializer;
/// The TOML deserializer
pub mod toml_deserializer;
/// The YAML deserializer
pub mod yaml_deserializer;

/// Helper function used to get a deserializer for a given file (by file extension)
pub mod deserializer;
