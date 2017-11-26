use description::name::Name;
use description::name::HasName;
use description::name::HasRoute;
use description::datatype::DataType;
use loader::loader::Validate;
use std::fmt;

#[derive(Deserialize, Debug)]
pub struct IO {
    pub name: Name,
    pub datatype: Name,
    #[serde(skip_deserializing)]
    pub route: String
}

// TODO figure out how to have this derived automatically for types needing it
impl HasName for IO {
    fn name(&self) -> &str {
        &self.name[..]
    }
}

impl HasRoute for IO {
    fn route(&self) -> &str {
        &self.route[..]
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

impl fmt::Display for IO {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\tname: \t\t{}\n\t\t\t\t\troute: \t\t{}\n\t\t\t\t\tdatatype: \t{}\n",
               self.name, self.route, self.datatype)
    }
}