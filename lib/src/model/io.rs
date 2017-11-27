use model::name::Name;
use model::name::HasName;
use model::name::HasRoute;
use model::datatype::DataType;
use model::datatype::HasDataType;
use loader::loader::Validate;
use std::fmt;

#[derive(Deserialize, Debug)]
pub struct IO {
    pub name: Name,
    #[serde(rename = "type")]
    pub datatype: DataType,
    #[serde(skip_deserializing)]
    pub route: String
}

// TODO figure out how to have this derived automatically for types needing it
impl HasName for IO {
    fn name(&self) -> &str {
        &self.name[..]
    }
}

impl HasDataType for IO {
    fn datatype(&self) -> &str {
        &self.datatype[..]
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
        self.datatype.validate()
    }
}

impl fmt::Display for IO {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "name: \t\t{}\n\t\t\t\t\troute: \t\t{}\n\t\t\t\t\tdatatype: \t{}\n",
               self.name, self.route, self.datatype)
    }
}