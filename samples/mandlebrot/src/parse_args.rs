
use std;
use create_complex::create_complex;
use parse_pair::parse_pair;
use std::path::PathBuf;
use num::Complex;
use std::io::Write;

pub fn parse_args() -> (PathBuf, (usize, usize), Complex<f64>, Complex<f64>) {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 5 {
        writeln!(std::io::stderr(), "Usage:   {} FILE PIXELS UPPERLEFT LOWERRIGHT", args[0]).unwrap();
        writeln!(std::io::stderr(), "Example: {} mandel.png 1000x750 -1.2,0.35 -1,0.20", args[0]).unwrap();
        std::process::exit(1);
    }

    let _executable_name = &args[0];

    let filename = PathBuf::from(&args[1]);

    let bounds = parse_pair(&args[2], "x").expect("error parsing image dimensions");

    let upper_left_args: (f64, f64) = parse_pair(&args[3], ",").expect("error parsing upper left corner point");
    let upper_left = create_complex(upper_left_args.0, upper_left_args.1);

    let lower_right_args = parse_pair(&args[4], ",").expect("error parsing lower rightcorner point");
    let lower_right = create_complex(lower_right_args.0, lower_right_args.1);

    (filename, bounds, upper_left, lower_right)
}