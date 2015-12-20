use description::name::{Name, Validates};
use description::datatype::DataType;
use parser::parser;

pub struct IO {
	pub name: Name, // Input/Output points on Entities, Values and Flows have unique names
	data_type: DataType,
// TODO consider adding references to the source& dest objects that is built when parsed
}

impl IO {
	pub fn validate_fields(&self) -> parser::Result {
		self.name.validate_fields("IO") // TODO early return here
		// TODO validate datatype
	}
}

pub type Input = IO;

pub type Output = IO;

pub struct InputOutput {
	pub name: Name,
	input_data_type: DataType,
	output_data_type: DataType,
}

impl InputOutput {
	fn validate_fields(&self) -> parser::Result {
		// TODO early return on failure
		self.name.validate_fields("InputOutput")
		// TODO validate data types
	}
}

pub type OutputInput = InputOutput;

pub struct IOSet {
	inputs: Vec<IO>,
	outputs: Vec<IO>,
	input_outputs: Vec<InputOutput>,
	output_inputs: Vec<OutputInput>,
}

impl IOSet {
	pub fn new(inputs: Vec<IO>, outputs: Vec<IO>, input_outputs: Vec<InputOutput>, output_inputs: Vec<OutputInput>) -> IOSet {
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