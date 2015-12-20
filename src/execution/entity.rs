use description::io;
use execution::value::Value;

pub trait HasInput {
	fn receive(&self, input: io::IO, value: Value);
}

pub trait HasOutput {
	// An entity with this trait may call the system to provide data
	// at the moment modelled as if we ask for it, but maybe it will be able
	// to call the system when data is available
	fn provide(&self, output: io::IO) -> Value;
}

pub trait HasInputOutput {
	fn receive_and_provide(&self, input_output: io::InputOutput, input_value: Value) -> Value;
}

pub trait HasOutputInput {
	fn provide_and_receive(&self, output_input: io::OutputInput, outputValue: Value) -> Value;
}