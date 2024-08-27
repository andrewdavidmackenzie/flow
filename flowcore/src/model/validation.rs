use crate::errors::Result;

/// The `Validate` trait used to check the elements deserialized from file obey all constraints
///
/// Some constraints cannot be expressed in the struct definition in `serde`
pub trait Validate {
    /// Validate that a deserialized model data structure is valid for use
    ///
    /// # Errors
    ///
    /// Will return `Err` if the value is not valid for this type
    fn validate(&self) -> Result<()>;
}