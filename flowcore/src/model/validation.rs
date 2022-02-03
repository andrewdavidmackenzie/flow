use crate::errors::*;

/// Many structs in the model implement the `Validate` method which is used to check the
/// description deserialized from file obeys some additional constraints that cannot be expressed
/// in the struct definition in `serde`
pub trait Validate {
    /// Validate that a deserialized model data structure is valid for use
    fn validate(&self) -> Result<()>;
}