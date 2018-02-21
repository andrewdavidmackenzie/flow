use model::name::Name;
use model::name::HasName;
use model::connection::HasRoute;
use model::datatype::DataType;
use model::datatype::HasDataType;
use loader::loader::Validate;
use model::connection::Route;

use std::fmt;

#[derive(Deserialize)]
pub struct Value {
    pub name: Name,
    #[serde(rename = "type")]
    pub datatype: DataType,
    #[serde(skip_deserializing)]
    pub route: Route,
    pub value: Option<String>,
}

impl HasName for Value {
    fn name(&self) -> &str {
        &self.name[..]
    }
}

impl HasDataType for Value {
    fn datatype(&self) -> &str {
        &self.datatype[..]
    }
}

impl HasRoute for Value {
    fn route(&self) -> &str {
        &self.route[..]
    }
}

impl Validate for Value {
    fn validate(&self) -> Result<(), String> {
        if let Some(ref value) = self.value {
            value.validate()?;
        }
        self.datatype.validate()
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\tname: \t\t{}\n\t\t\t\t\troute: \t\t{}\n\t\t\t\t\tdatatype: \t{}\n",
               self.name, self.route, self.datatype).unwrap();
        if let Some(ref value) = self.value {
            write!(f, "\t\t\t\t\tvalue: \t\t{}", value).unwrap();
        }
        Ok(())
    }
}