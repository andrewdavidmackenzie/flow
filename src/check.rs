extern crate getopts;
use getopts::Options;
use std::env;

extern crate flow;
use flow::description::validator::validate as validate;
// TODO use as

#[cfg(test)]
mod tests {
	use super::print_usage;
	use getopts::Options;

	#[test]
	fn can_print_usage() {
		let opts = Options::new();
		print_usage("test", opts);
	}
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

	validate(&input);
}