use datatype;

struct IO {
	name: String,
	dataType: DataType,
}

type Input = IO;

type Output = IO;

struct InputOutput {
	input: Input,
	output: Output,
}

struct OutputInput {
	output: Output,
	input: Input,
}