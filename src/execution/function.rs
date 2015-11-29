use description::io::Input;
use description::io::Output;
use execution::value::Value;

pub trait Function {
	fn run(Vec<Value>) -> Vec<Value>;
}