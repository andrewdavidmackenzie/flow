use input;
use output;
use value;
use connection;
use request;
use value;

struct Flow {
	name: String,
	inputs: Vec<Input>,
	outputs: Vec<Ouput>,
	inputOutputs: Vec<InputOutput>,
	outputInputs: Vec<OuputInput>,
	values: Vec<Value>,
	flows: Vec<Flow>, 				// sub-flows
	functions: Vec<Function>,
	connections: Vec<Connection>,
	requests: Vec<Requests>,
}