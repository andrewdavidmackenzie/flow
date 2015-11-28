use io;
use value;

trait HasInput {
	fn receive(&self, input: IO, value: Value);
}

trait HasOutput {
	// An entity with this trait may call the system to provide data
	// at the moment modelled as if we ask for it, but maybe it will be able
	// to call the system when data is available
	fn provide(&self, output: IO) -> Value;
}

trait HasInputOutput {
	fn receiveAndProvide(&self, input: IO, value: Value, output: IO) -> Value;
}

trait HasOutputInput {
	// An entity with this trait may call the system to make a request
	// from which it expects a response
}