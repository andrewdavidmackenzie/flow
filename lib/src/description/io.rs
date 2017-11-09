use description::name::{Name, Validates};
use description::datatype::DataType;
use parser::parser;

pub struct IO<'a> {
	pub name: Name<'a>, // Input/Output points on Entities, Values and Flows have unique names
	data_type: DataType<'a>,
// TODO consider adding references to the source& dest objects that is built when parsed
}

impl<'a> IO<'a> {
	pub fn validate_fields(&self) -> parser::Result {
		self.name.validate_fields() // TODO early return here
		// TODO validate datatype
	}
}

pub type Input<'a> = IO<'a>;

pub type Output<'a> = IO<'a>;

pub struct InputOutput<'a> {
	pub name: Name<'a>,
	input_data_type: DataType<'a>,
	output_data_type: DataType<'a>,
}

impl<'a> InputOutput<'a> {
	fn validate_fields(&self) -> parser::Result {
		// TODO early return on failure
		self.name.validate_fields()
		// TODO validate data types
	}
}

pub type OutputInput<'a> = InputOutput<'a>;

pub struct IOSet<'a> {
	inputs: Vec<IO<'a>>,
	outputs: Vec<IO<'a>>,
	input_outputs: Vec<InputOutput<'a>>,
	output_inputs: Vec<OutputInput<'a>>,
}

/*
Implement a default set of empty vectors for IOSet, then any instances just need to specify  the ones they create
 */
impl<'a> Default for IOSet<'a> {
	fn default () -> IOSet<'a> {
		IOSet {
			inputs : vec![],
			outputs : vec![],
			input_outputs : vec![],
			output_inputs : vec![]
		}
	}
}

impl<'a> IOSet<'a> {
	pub fn new(inputs: Vec<IO<'a>>, outputs: Vec<IO<'a>>, input_outputs: Vec<InputOutput<'a>>, output_inputs: Vec<OutputInput<'a>>) -> IOSet<'a> {
		IOSet {
			inputs: inputs,
			outputs: outputs,
			input_outputs: input_outputs,
			output_inputs: output_inputs,
		}
	}

	pub fn validate_fields(&self) -> parser::Result {
		// TODO early return on failure
		for input in &self.inputs {
			input.validate_fields();
		}
		for output in &self.outputs {
			output.validate_fields();
		}
		for input_output in &self.input_outputs {
			input_output.validate_fields();
		}
		for output_input in &self.output_inputs {
			output_input.validate_fields();
		}
		parser::Result::Valid
	}
}