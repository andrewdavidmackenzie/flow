use description::io::{Output, Input};
use execution::function::Function;
use execution::value::Value;

struct Add;

impl Add {
	fn run(inputs: Vec<Value>) -> Value {
		Value {
			value: "2".to_string(),
		}// TODO
	}
}