use driver;
use datatype;

struct Entity {
	name: String,
	inputs: Vec<IO>,
	outputs: Vec<IO>,
	inputOutputs: Vec<InputOutput>,
	outputInputs: Vec<OututInput>,
}