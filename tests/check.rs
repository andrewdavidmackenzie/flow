extern crate check;

#[test]
fn can_print_usage() {
	let opts = Options::new();
	print_usage("test", opts);
}