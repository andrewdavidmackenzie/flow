use description::datatype::DataType;

pub struct IO {
	name: String,
	dataType: DataType,
}

pub type Input = IO;

pub type Output = IO;

pub struct InputOutput {
	input: Input,
	output: Output,
}

pub struct OutputInput {
	output: Output,
	input: Input,
}