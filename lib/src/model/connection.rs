use model::name::Name;
use loader::loader::Validate;

use std::fmt;

#[derive(Deserialize, Debug)]
pub struct Connection {
    pub name: Option<Name>,
    pub from: Name,
    #[serde(skip_deserializing)]
    pub from_route: String,
    pub to: Name,
    #[serde(skip_deserializing)]
    pub to_route: String
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