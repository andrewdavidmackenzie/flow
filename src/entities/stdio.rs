use std::io;
use std::io::Write;

use description::io::{IO, InputOutput, OutputInput};
use execution::value::Value;
use execution::entity::{HasInput, HasOutput, HasInputOutput};

struct STDIO;

impl HasInput for STDIO {
	fn receive(&self, input: IO, value: Value) {
		match input.name.as_ref() {
			"stdout" => println!("{}", value.value),
			// TODO
//			"stderr" => try!(io::stderr().write(b"hello world\n")),
			_ => panic!("STDIO does not have an input called '{}'", input.name),
			}
	}
}

impl HasOutput for STDIO {
	fn provide(&self, output: IO) -> Value {
		match output.name.as_ref() {
			"stdin" => {
				let mut input = String::new();
				io::stdin().read_line(&mut input).ok();
				Value {
				value: input,
				} // TODO
			},
			_ => panic!("STDIO does not have an ouput called '{}'", output.name),
		}
	}
}

/*
	After receiving the input value on this input, this entity will block and not process
	any other inputs, or generate any other outputs, until it has produced the output expected
 */
impl HasInputOutput for STDIO {
	fn receive_and_provide(&self, input_output: InputOutput, input_value: Value) -> Value {
		match input_output.name.as_ref() {
			"prompt" => {
				println!("{}", input_value.value);
				// TODO output to stdout should be blocked until we read and provide a response
				let mut input = String::new();
				io::stdin().read_line(&mut input).ok();
				Value {
					value: input,
				} // TODO
			},
			_ => panic!("STDIO does not have an InputOutput called '{}'", input_output.name),
		}
	}
}
