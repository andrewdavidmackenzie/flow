use description::io::Output;
use description::io::Input;
use execution::function::Function;
use execution::value::Value;

struct Add;

impl Function for Add {
	fn run(inputs: Vec<Value>) -> Vec<Value> {
		let r = Value::new("2");
		return vec![r]; // TODO
	}
}