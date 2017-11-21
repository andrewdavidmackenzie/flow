use loader::loader::Validate;use description::name::Name;
use description::io::IO;

/*
	Unidirectional connection between an Output and an Input
	Can only carry a single datatype
 */
pub struct Connection {
	name: Name,
// change to references to ensure they refer to real Input/Output and not copy them
	from: IO,
	to: IO,
}

impl Connection {
	// TODO change to references later
	fn new(name: Name, from: IO, to: IO) -> Connection {
		Connection {
			name: name,
			from: from,
			to : to,
		}
	}
}

impl Validate for Connection {
    fn validate(&self) -> Result<(), String> {
        self.name.validate() // TODO early return

        // Validate other fields exist and are valid syntax
        // TODO validate directions match the end points
        // TODO Validate the two types are the same or can be inferred
    }
}

pub struct ConnectionSet {
	connections: Vec<Connection>
}

/*
From the Connection set, return a subset that are connected to a specific name
 */
/*
impl ConnectionSet {
	pub fn new(connections: Vec<Connection>, requests: Vec<Request>) -> ConnectionSet {
		ConnectionSet {
			connections: connections,
			requests: requests,
		}
	}

	pub fn get_subset(&self, name: &String) -> ConnectionSet {
		// TODO
		ConnectionSet::new(vec![], vec![])
	}

	pub fn validate_fields(&self) -> parser::Result  {
		for connection in &self.connections {
			connection.validate_fields(); // TODO early return
		}

		parser::Result::Valid
	}

	pub fn check(connection_set: &ConnectionSet, io_sets: &Vec<&IOSet>, values: &Vec<Value>)
    -> parser::Result {
		// TODO
		// for each connection
		// connected at both ends to something passed in, directions and types match
		// 		validateConnection in itself, not to subflow
		parser::Result::Valid
	}
}
*/