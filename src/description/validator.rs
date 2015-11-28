mod description {

pub fn validate(filename: String) {
	println!("Checking consistency of flow models");

	/*
	load the context flow
	check that it has must fields
		name

	for each source and sink
		check that they have must fields
			- name and is valid name
		check that drivers can be found for them

	for each embedded flow (only one) check
		file exists and can be read
		for outputs and inputs
			check they have must fields
				names and is valid name
				type and is a valid type
			check that destination exists here
			check that destination can generate/accept the data type of the input/output
			check that they coincide with the inputs and outputs listed in the file


	*/
}
}
