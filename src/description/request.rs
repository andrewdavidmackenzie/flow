use datatype;
use io;

/*
	Bidirectional request from one IO to another with a datatype for the
	request and another datatype for the response.
 */
struct Request  {
	name: String,
	from: &OutputInput,
	requestDataType: DataType,
	to: &InputOutput,
	responseDataType: DataType,
}