use datatype;
use io;

/*
	Unidirectional connection between two IOs of a single datatype
 */
struct Connection {
	name: String,
	dataType: DataType,
	from: &IO,
	to: &IO,
}