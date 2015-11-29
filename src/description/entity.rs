use description::io::IO;
use description::io::InputOutput;
use description::io::OutputInput;

struct Entity {
	name: String,
	inputs: Vec<IO>,
	outputs: Vec<IO>,
	inputOutputs: Vec<InputOutput>,
	outputInputs: Vec<OutputInput>,
}