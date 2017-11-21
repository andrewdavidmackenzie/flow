use loader::loader::Validate;
use description::datatype::DataType;
use description::name::Name;

pub struct IO {
	pub name: Name, // Input/Output points on Entities, Flows, Sinks, Sources have unique names
	data_type: DataType,
}

impl Validate for IO {
	fn validate(&self) -> Result<(), String> {
		self.name.validate() // TODO early return here try!() ????
		// TODO validate datatype exists?
	}
}

pub struct IOSet {
	ios: Vec<IO>
}

/*
Implement a default set of empty vectors for IOSet, then any instances just need to specify  the ones they create
 */
impl Default for IOSet {
	fn default () -> IOSet {
		IOSet {
			ios : vec![]
		}
	}
}

impl IOSet {
	pub fn new(ios: Vec<IO>) -> IOSet {
		IOSet {
			ios: ios
		}
	}
}

impl Validate for IOSet {
    fn validate(&self) -> Result<(), String> {
        // TODO early return on failure
        for io in &self.ios {
            io.validate();
        }

        Ok(())
    }
}