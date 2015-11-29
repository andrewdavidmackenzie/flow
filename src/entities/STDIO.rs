use std::io;
use description::io::IO;
use execution::value::Value;
use execution::entity::HasInput;
use execution::entity::HasOutput;
use execution::entity::HasInputOutput;

struct STDIO;

// for now just generic methods, but these should probably be repeated for
// each of the inputs / outputs an entity has, by name

impl HasInput for STDIO {
	fn receive(&self, input: IO, value: Value) {
//		println!("");  // TODO
	}
}

impl HasOutput for STDIO {
	fn provide(&self, output: IO) -> Value {
		/* TODO
		let mut buffer = String::new();
		io::stdin().read_line(&mut buffer);
		Value::new(&buffer);
		*/
		Value::new("result")
	}
}

/*
	After receiving the input value on this input, this entity will block and not process
	any other inputs, or generate any other outputs, until it has produced the output expected
 */
impl HasInputOutput for STDIO {
	fn receiveAndProvide(&self, input: IO, value: Value, output: IO) -> Value {
// TODO		STDIO::receive(&self, input, value);
		STDIO::provide(&self, output)
	}
}
