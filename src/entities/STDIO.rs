use std::io;
use io;
use value;
use entity;

// for now just generic methods, but these should probably be repeated for
// each of the inputs / outputs an entity has, by name

impl HasInput for Entity {
	fn receive(&self, input: IO, value: Value) {
		println!(input);
	}
}

impl HasOutput for Entity {
	fn provide(&self, output: IO) -> Value {
		let mut buffer = String::new();
		try!(io::stdin().read_to_string(&mut buffer));
		buffer;
	}
}

/*
	After receiving the input value on this input, this entity will block and not process
	any other inputs, or generate any other outputs, until it has produced the output expected
 */
impl HasInputOutput for Entity {
	fn receiveAndProvide(&self, input: IO, value: Value, output: IO) -> Value {
		println!(input);
		let mut buffer = String::new();
		try!(io::stdin().read_to_string(&mut buffer));
		buffer;
	}
}
