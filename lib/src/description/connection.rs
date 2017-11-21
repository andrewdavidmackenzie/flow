use loader::loader::Validate;use description::name::Name;
use description::io::IORef;
use std::fmt;

#[derive(Deserialize, Debug)]
pub struct Connection {
    pub name: Option<Name>,
    from: IORef,
    to: IORef,
}

impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Connection:\n\tname: {:?}\n\tfrom: {:?}\n\tto: {:?}", self.name, self.from, self.to)
    }
}

impl Connection {
	// TODO change to references later
	fn new(name: Option<Name>, from: IORef, to: IORef) -> Connection {
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