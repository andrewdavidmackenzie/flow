use description::name::Name;
use loader::loader::Validate;

use std::fmt;

#[derive(Deserialize, Debug)]
pub struct Connection {
    pub name: Option<Name>,
    pub from: Name,
    pub to: Name,
}

impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Connection:\n\tname: {:?}\n\tfrom: {:?}\n\tto: {:?}", self.name, self.from, self.to)
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