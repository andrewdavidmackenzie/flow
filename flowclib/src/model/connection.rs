use model::name::Name;
use loader::loader::Validate;
use model::io::IO;

use std::fmt;

pub type Route = String;

pub trait HasRoute {
    fn route(&self) -> &str;
}

#[derive(Deserialize, Debug, Clone)]
pub struct Connection {
    pub name: Option<Name>,
    pub from: Route,
    pub to: Route,

    #[serde(skip_deserializing)]
    pub from_io: IO,
    #[serde(skip_deserializing)]
    pub to_io: IO
}

#[derive(Debug)]
pub enum Direction {
    FROM, TO
}

impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (self.from_io.flow_io, self.to_io.flow_io) {
            (true, true)   => write!(f, "(f){} --> (f){}", self.from_io.route, self.to_io.route),
            (true, false)  => write!(f, "(f){} --> {}", self.from_io.route, self.to_io.route),
            (false, true)  => write!(f, "{} --> (f){}", self.from_io.route, self.to_io.route),
            (false, false) => write!(f, "{} --> {}", self.from_io.route, self.to_io.route)
        }
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