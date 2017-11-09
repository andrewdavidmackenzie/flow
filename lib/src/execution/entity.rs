use description::io;
use execution::value::Value;

pub trait HasInput {
	fn receive(&self, value: Value);
}

// An entity with this trait may call the system to provide data
// at the moment modelled as if we ask for it, but maybe it will be able
// to call the system when data is available
pub trait HasOutput {
	fn provide(&self) -> Value;
}

pub trait HasInputOutput {
	fn receive_and_provide(&self, input_value: Value) -> Value;
}

pub trait HasOutputInput {
	fn provide_and_receive(&self, outputValue: Value) -> Value;
}