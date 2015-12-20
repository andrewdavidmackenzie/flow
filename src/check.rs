extern crate getopts;
use getopts::Options;
use std::env;

extern crate flow;
use flow::parser::parser;

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
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    /* TODO
    let input = if !matches.free.is_empty()
    } else {
    // TODO look for file with context or flow extension in the current directory
    // if only one, then run on it.
    print_usage(&program, opts);
    };
    */

    let input = matches.free[0].clone();
    match parser::load(&input, true) {
        parser::Result::ContextLoaded(context) => println!("'{}' parsed and validated correctly", context.name),
        parser::Result::FlowLoaded(flow) => println!("'{}' parsed and validated correctly", flow.name),
        parser::Result::Error(error) => println!("{}", error),
        parser::Result::Valid => println!("Shouldn't happen"),
    }
}