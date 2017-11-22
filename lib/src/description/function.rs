use description::name::Name;
use description::connection::IO;
use loader::loader::Validate;

#[derive(Deserialize, Debug)]
pub struct FunctionRef {
	name: Name,
	source: String
}

#[derive(Deserialize, Debug)]
pub struct Function {
    name: Name,
    input: Option<Vec<IO>>,
    output: Option<Vec<IO>>,
    implementation: Option<String> // TODO for now
}

impl Validate for Function {
	fn validate(&self) -> Result<(), String> {
		self.name.validate() // TODO early return
        // TODO validate all the rest
	}
}