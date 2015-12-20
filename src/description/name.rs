//#![feature(test)] // enables this unstable feature.
//extern crate test;

use parser::parser;

pub type Name = String;

// Define a trait to be able to add a function to String
pub trait Validates {
	fn validate_fields(&self, type_name: &str) -> parser::Result;
}

impl Validates for Name {
	fn validate_fields(&self, type_name: &str) -> parser::Result {
		if self.is_empty() {
			return parser::Result::Error(format!("{} cannot have an empty or whitespace name", type_name));
		}
		parser::Result::Valid
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use test::Bencher;

	#[test]
	fn can_validate() {
		let name = Name("test");
		assert!(name.validate("Name"));
	}

	#[bench]
	fn bench_validate(b: &mut Bencher) {
		b.iter(|| {
			let name = Name("test");
			name.validate("Name")  // return it to avoid the optimizer removing it
		});
	}
}