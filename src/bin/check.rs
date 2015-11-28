extern crate getopts;
use getopts::Options;
use std::env;

fn check(inp: &str) {
	println!("Checking correctness of flow '{}'", inp);

	//	description::validator::validate("filename");

	/*

	load flow definition from file specified in arguments
	- load any referenced to included flows also

	construct overall list of functions
	- check complete and no unattached input flows?

	construct initial list of all functions able to produce output
	- start from external sources at level 0

	do
	- identify all functions which receive input from active sources
	- execute all those functions
	- functions producing output added to list of active sources
	while functions pending input

	*/

}

fn print_usage(program: &str, opts: Options) {
	let brief = format!("Usage: {} FILE [options]", program);
	print!("{}", opts.usage(&brief));
}

fn main() {
	let args: Vec<String> = env::args().collect();
	let program = args[0].clone();

	let mut opts = Options::new();
	opts.optflag("h", "help", "print this help menu");
	let matches = match opts.parse(&args[1..]) {
			Ok(m) => { m}
			Err(f) => { panic!(f.to_string())}
		};

	if matches.opt_present("h") {
			print_usage(&program, opts);
			return;
	}

	let input = if !matches.free.is_empty() {
			matches.free[0].clone()
	} else {
			print_usage(&program, opts);
			return;
	};

	check(&input);
}