use std::io;
use std::io::Write;

use description::io::{IO, Input, Output, InputOutput, OutputInput, IOSet};
use description::entity::Entity;
use execution::value::Value;
use execution::entity::{HasInput, HasOutput, HasInputOutput};


// TODO the runtime that loads all these should build a table of name -> objects
// and then find them at run time by name

// TODO this should be able to generate or be validatable again the description in stdio.entity

const PROMPT: InputOutput = InputOutput {
	name: "prompt",
	input_data_type: "String",
	output_data_type: "String"
};

const STDOUT: Input = Input {
	name: "stdout",
	data_type: "String"
};

const STDERR: Input = Input {
	name: "stderr",
	data_type: "String"
};

const STDIN: Output = Output {
	name: "stdin",
	data_type: "String"
};

const INPUTS: Vec<IO<'a>> = vec![STDOUT, STDERR];
const OUTPUTS: Vec<IO<'a>> = vec![STDIN];
const INPUTS_OUTPUTS: Vec<InputOutput<'a>> = vec![PROMPT];

const IOSET: IOSet = IOSet {
	inputs: INPUTS,
	outputs: OUTPUTS,
	input_outputs: INPUTS_OUTPUTS
};

const STDIO: Entity = Entity {
	name: "stdio",
	ios: IOSET
};

impl HasInput for STDOUT {
	fn receive(&self, value: Value) {
		// TODO use value.value
		try!(io::stdout().write(b"hello world\n"));
	}
}

impl HasInput for STDERR {
	fn receive(&self, value: Value) {
		// TODO use value.value
		try!(io::stderr().write(b"hello world\n"));
	}
}

impl HasOutput for STDIN {
	fn provide(&self) -> Value {
		let mut input = String::new();
		io::stdin().read_line(&mut input).ok();
		Value {
			value: input,
		} // TODO
	}
}

// This will not *really* block, just not schedule the flow waiting for input until it's input is satisfied.
impl HasInputOutput for PROMPT {
	fn receive_and_provide(&self, input_value: Value) -> Value {
		STDOUT.receive(input_value);
		STDIN.provide();
	}
}