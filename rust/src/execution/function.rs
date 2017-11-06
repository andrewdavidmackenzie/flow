use description::io::{Input, Output};
use execution::value::Value;

pub trait Function {
	fn run(Vec<Value>) -> Vec<Value>;
}