use serde_json::Value as JsonValue;
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
    pub value: Option<JsonValue>,
    #[serde(skip_deserializing)]
    pub output_routes: Vec<(usize, usize)>,
    #[serde(skip_deserializing)]
    pub id: usize,
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
        self.datatype.validate()
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\tname: \t\t{}\n\t\t\t\t\troute: \t\t{}\n\t\t\t\t\tdatatype: \t{}\n",
               self.name, self.route, self.datatype).unwrap();
        if self.value.is_some() {
            write!(f, "\t\t\t\t\tvalue: \t\t{:?}", self.value).unwrap();
        }
        Ok(())
    }
}