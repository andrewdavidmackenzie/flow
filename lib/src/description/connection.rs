use description::name::Name;
use description::datatype::DataType;
use loader::loader::Validate;

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
    fn validate(&self) -> Result<(), String> {
        //self.name.unwrap().validate() // TODO early return
        Ok(())

        // check other fields exist and are valid syntax
        // TODO check directions match the end points
        // TODO check the two types are the same or can be inferred
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

        // TODO check datatype
        Ok(())
    }
}