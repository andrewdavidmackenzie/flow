//! deserializer modules provides deserializers from different file formats
//!
//! and also helper methods to get a deserializer based on the file extension of a file referred 
//! to by a Url

// The JSON deserializer
mod json_deserializer;
// The TOML deserializer
mod toml_deserializer;
// The YAML deserializer
mod yaml_deserializer;

/// Helper function used to get a deserializer for a given file (by file extension)
pub mod deserializer;
