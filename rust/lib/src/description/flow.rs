use description::name::{Name, Validates};
use description::connection::ConnectionSet;
use description::io::IOSet;
use description::value::Value;
use description::function::Function;
use parser::parser;

pub struct Flow {
	pub name: Name,
	source_path: String,
	flows: Vec<(String, String, Box<Flow>)>,
	connection_set: ConnectionSet,
	ios: IOSet,
	values: Vec<Value>,
	functions: Vec<Function>,
}

impl Flow {
	pub fn new(name: String, path: &str, flows: Vec<(String, String, Box<Flow>)>,
		   connection_set: ConnectionSet, ios: IOSet, values: Vec<Value>, functions: Vec<Function>)
	-> Flow {
		Flow {
			name: name,
			source_path: path.to_string(),
			flows: flows,
			ios: ios,
			values: values,
			functions: functions,
			connection_set: connection_set,
		}
	}

	pub fn validate_fields(&self) -> parser::Result {
		self.name.validate_fields("Flow"); // TODO early return

		// validate flows (name only and valid path)

		// Validate all IOs are valid names and types

		// Validate values

		// validate functions

		// validate connections are all valid

        // TODO
        parser::Result::Valid
	}

    pub fn load_sub_flows(&mut self) -> parser::Result {
        for &(_, ref path, _) in self.flows.iter() {
            // TODO FIX
            let load_result = parser::load(path.as_ref(), false);
//            let load_result = parser::Result::Valid;
            match load_result {
                parser::Result::FlowLoaded(subflow) => {
                    // TODO set reference to child flow
                    return parser::Result::Valid;
                },
                _ => {},
            }
            return load_result;
        }
        parser::Result::Valid
    }

    pub fn validate_connections(&self) -> parser::Result {
        let mut io_sets: Vec<&IOSet> = vec![];

        for &(_, _, ref flow) in self.flows.iter() {
            // add subflow's ioset to the set of IOs to check connections to
            io_sets.push(&(flow.ios));
        }

        // Add the IOSets of all functions
        for function in &self.functions {
            io_sets.push(&function.ios);
        }

        // Add the input/outputs of this flow to parent
        io_sets.push(&self.ios);

        // for each check connections with their ioset
        ConnectionSet::check(&self.connection_set, &io_sets, &self.values)
    }

    pub fn subflow(&mut self) -> parser::Result {
        for &mut(_, _, ref mut subflow) in self.flows.iter_mut() {
            subflow.validate_fields(); // TODO early return
            subflow.load_sub_flows(); // TODO early return
            subflow.validate_connections(); // TODO early return
            subflow.subflow();
        }

        // TODO FIX
        parser::Result::Valid
    }
}