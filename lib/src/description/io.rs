use loader::loader::Validate;

use description::name::Name;
//use description::datatype::DataType;

pub struct IO {
	pub name: Name, // Input/Output points on Entities, Values and Flows have unique names
//	data_type: DataType<'a>,
// TODO consider adding references to the source& dest objects that is built when parsed
}

impl Validate for IO {
	fn validate(&self) -> Result<(), String> {
		self.name.validate() // TODO early return here try!() ????
		// TODO validate datatype
	}
}

pub type Input = IO;

pub type Output = IO;

pub struct InputOutput {
	pub name: Name,
//	input_data_type: DataType<'a>,
//	output_data_type: DataType<'a>,
}

impl Validate for InputOutput {
	fn validate(&self) -> Result<(), String> {
		// TODO early return on failure
		self.name.validate()
		// TODO validate data types
	}
}

pub type OutputInput = InputOutput;

pub struct IOSet {
//	inputs: Vec<IO<'a>>,
//	outputs: Vec<IO<'a>>,
//	input_outputs: Vec<InputOutput<'a>>,
//	output_inputs: Vec<OutputInput<'a>>,
}

/*
Implement a default set of empty vectors for IOSet, then any instances just need to specify  the ones they create
 */
impl Default for IOSet {
	fn default () -> IOSet {
		IOSet {
//			inputs : vec![],
//			outputs : vec![],
//			input_outputs : vec![],
//			output_inputs : vec![]
		}
	}
}

impl IOSet {
	pub fn new(inputs: Vec<IO>, outputs: Vec<IO>, input_outputs: Vec<InputOutput>,
			   output_inputs: Vec<OutputInput>) -> IOSet {
		IOSet {
//			inputs: inputs,
//			outputs: outputs,
//			input_outputs: input_outputs,
//			output_inputs: output_inputs,
		}
	}
}

impl Validate for IOSet {
    fn validate(&self) -> Result<(), String> {
        // TODO early return on failure
/*        for input in &self.inputs {
            input.validate();
        }
        for output in &self.outputs {
            output.validate();
        }
        for input_output in &self.input_outputs {
            input_output.validate();
        }
        for output_input in &self.output_inputs {
            output_input.validate();
        }*/
        Ok(())
    }
}