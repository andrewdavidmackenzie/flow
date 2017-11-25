use description::name::Name;
use description::name::Named;
use description::datatype::DataType;
use loader::loader::Validate;

use std::fmt;

#[derive(Deserialize, Debug)]
pub struct IO {
    pub name: Name,
    pub datatype: Name
}

// TODO figure out how to have this derived automatically for types needing it
impl Named for IO {
    fn name(&self) -> &str {
        &self.name[..]
    }
}

impl Validate for IO {
    fn validate(&self) -> Result<(), String> {
        self.name.validate()?;
        self.datatype.validate()?;
        let dt_slice: &str = &self.datatype[..];
        DataType::valid_type(dt_slice)?;
        Ok(())
    }
}