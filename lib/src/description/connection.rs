use loader::loader::Validate;
use description::name::Name;
use description::datatype::DataType;

use std::fmt;

#[derive(Deserialize, Debug)]
pub struct Connection {
    pub name: Option<Name>,
    from: IOName,
    to: IOName,
}

impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Connection:\n\tname: {:?}\n\tfrom: {:?}\n\tto: {:?}", self.name, self.from, self.to)
    }
}

impl Connection {
	// TODO change to references later
	fn new(name: Option<Name>, from: IOName, to: IOName) -> Connection {
		Connection {
			name: name,
			from: from,
			to : to,
		}
	}
}

impl Validate for Connection {
    fn validate(&self) -> Result<(), String> {
        //self.name.unwrap().validate() // TODO early return
        Ok(())

        // Validate other fields exist and are valid syntax
        // TODO validate directions match the end points
        // TODO Validate the two types are the same or can be inferred
    }
}

pub type IOName = String;

#[derive(Deserialize, Debug)]
pub struct IO {
    pub name: IOName,
    pub datatype: DataType
}

impl Validate for IO {
    fn validate(&self) -> Result<(), String> {
        self.name.validate()?;

        // TODO validate datatype
        Ok(())
    }
}