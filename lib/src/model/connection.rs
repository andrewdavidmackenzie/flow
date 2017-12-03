use model::name::Name;
use model::datatype::DataType;
use loader::loader::Validate;

use std::fmt;

pub type Route = String;

pub trait HasRoute {
    fn route(&self) -> &str;
}

// This trait should be implemented by objects that have collections of IO objects as inputs
// the method input_type should find an input by name and return the type it accepts
pub trait HasInputs {
    fn input_type(&self, input_name: &Name) -> Result<DataType, String>;
}

#[derive(Deserialize, Debug)]
pub struct Connection {
    pub name: Option<Name>,
    pub from: Name,
    #[serde(skip_deserializing)]
    pub from_route: Route,
    #[serde(skip_deserializing)]
    pub from_type: DataType,
    pub to: Name,
    #[serde(skip_deserializing)]
    pub to_route: Route,
    #[serde(skip_deserializing)]
    pub to_type: DataType
}

impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} --> {}", self.from_route, self.to_route)
    }
}

impl Validate for Connection {
    // Called before everything is loaded and connected up to check all looks good
    fn validate(&self) -> Result<(), String> {
        if let Some(ref name) = self.name {
            name.validate()?;
        }
        self.from.validate()?;
        self.to.validate()
    }
}