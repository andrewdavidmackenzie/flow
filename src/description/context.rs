use source;
use destination;
use value;
use flow;
use connection;

struct Context {
	name: String,
	entities: Vec<Entity>,
	flow: Flow, // Only one sub-flow permitted in the Context Diagram
	connections: Vec<Connection>,
}