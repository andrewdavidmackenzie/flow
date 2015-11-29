/// # Example
/// ```
/// use flow::description::validator::validate;
///
/// flow::description::validator::validate("hello.context");
/// ```
pub fn validate(filename: &str) {
	println!("Checking correctness of flow '{}'", filename);

	// check the file exists and can be read

	/*
	read file

	if context
		validate name
		validate entities
		validate connections between flow and entities are correct
		validate contains a flow

	for each subflow
		build list of connections into/outof subflow
		file exists and can be read
		load sub-flow
			validate fields
			build list of connections into/outof this flow
		compare list of inputs/outputs
			for outputs and inputs
			check they have must fields
				names and is valid name
				type and is a valid type
			check that destination exists here
			check that destination can generate/accept the data type of the input/output
			check that they coincide with the inputs and outputs listed in the file

	check context
		check that it has must fields
			name

		for each source and sink
			check that they have must fields
				- name and is valid name
			check that drivers can be found for them

	*/
}
