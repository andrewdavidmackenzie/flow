#![feature(test)]

extern crate num;

use num::Complex;

extern crate image;
extern crate rayon;

use rayon::prelude::*;

use image::ColorType;
use image::png::PNGEncoder;

extern crate test;
extern crate dir_diff;
extern crate tempdir;

extern crate flowrlib;
#[macro_use]
extern crate serde_json;

mod escapes;
mod pixel_to_point;

use pixel_to_point::pixel_to_point;
use escapes::escapes;

use std::fs::File;
use std::path::PathBuf;
use std::str::FromStr;
use std::io::Result;
use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 5 {
        writeln!(std::io::stderr(), "Usage:   {} FILE PIXELS UPPERLEFT LOWERRIGHT", args[0]).unwrap();
        writeln!(std::io::stderr(), "Example: {} mandel.png 1000x750 -1.2,0.35 -1,0.20", args[0]).unwrap();
        std::process::exit(1);
    }

    let filename = PathBuf::from(&args[1]);

    let bounds = parse_pair(&args[2], 'x').expect("error parsing image dimensions");
    let upper_left = parse_complex(&args[3]).expect("error parsing upper left corner point");
    let lower_right = parse_complex(&args[4]).expect("error parsing lower right corner point");

    let mut pixels = vec![0; bounds.0 * bounds.1];

    render(&mut pixels, bounds, upper_left, lower_right);

    write_bitmap(&filename, &pixels, bounds).expect("error writing PNG file");
}

/// Parse the string 's' as a coordinate pair, like "400x600" or "1.0,0.5"
/// Specifically, 's' should have the form <left><sep><right> where <sep> is the character given by
/// the 'separator' argument, and <left> and <right> are both strings that can be parsed
/// by 'T::from_str'.
/// If 's' has the proper form, return 'Some<(x,y)>'.
/// If 's' doesn't parse correctly, return None.
fn parse_pair<T: FromStr>(s: &str, separator: char) -> Option<(T, T)> {
    match s.find(separator) {
        None => None,
        Some(index) => {
            match (T::from_str(&s[..index]), T::from_str(&s[index + 1..])) {
                (Ok(l), Ok(r)) => Some((l, r)),
                _ => None
            }
        }
    }
}

/// Parse a pair of floating-point numbers separated by a comma as a complex /// number.
fn parse_complex(s: &str) -> Option<Complex<f64>> {
    match parse_pair(s, ',') {
        Some((re, im)) => Some(Complex { re, im }),
        None => None
    }
}

fn render(pixels: &mut [u8], bounds: (usize, usize),
          upper_left: Complex<f64>, lower_right: Complex<f64>) {
    let rows = pixels.par_chunks_mut(bounds.0);
    rows.into_par_iter()
        .enumerate()
        .for_each(|(row_index, row)| {
            let row_bounds = (bounds.0, 1);
            let row_upper_left = pixel_to_point(bounds, (0, row_index),
                                                upper_left, lower_right);
            let row_lower_right = pixel_to_point(bounds, (bounds.0, row_index),
                                                 upper_left, lower_right);
            render_row(row, row_bounds, row_index, row_upper_left, row_lower_right);
        });
}

/// Render a row of the Mandlebrot set into a buffer of pixels
/// The 'bounds' argument gives the width and height of the buffer 'pixels' which holds one
/// grayscale pixel per byte. The 'upper_left' and 'lower_right' arguments specify points on the
/// complex plane corresponding to the upper left and lower right corners of the pixel buffer.
fn render_row(pixels: &mut [u8], bounds: (usize, usize), row_index: usize,
              upper_left: Complex<f64>, lower_right: Complex<f64>) {
    let mut offset: usize = 0;

    for column in 0..bounds.0 {
        // columns
        let point = pixel_to_point(bounds, (column, row_index), upper_left, lower_right);

        pixels[offset] = 255 - escapes(point, 255) as u8;
        offset += 1; // move forward a byte in the pixel buffer
    }
}

/// Write the buffer 'pixels', whose dimensions are given by 'bounds', to the file named 'filename'
fn write_bitmap(filename: &PathBuf, pixels: &[u8], bounds: (usize, usize)) -> Result<()> {
    let output = File::create(filename)?;

    let encoder = PNGEncoder::new(output);
    encoder.encode(&pixels, bounds.0 as u32, bounds.1 as u32,
                   ColorType::Gray(8))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    use tempdir::TempDir;

    #[test]
    fn test_pixel_to_point() {
        let upper_left = Complex { re: -1.0, im: 1.0 };
        let lower_right = Complex { re: 1.0, im: -1.0 };

        assert_eq!(pixel_to_point((100, 100), (25, 75),
                                  upper_left, lower_right),
                   Complex { re: -0.5, im: -0.5 });
    }

    #[test]
    fn test_parse_pair() {
        assert_eq!(parse_pair::<i32>("", ','), None);
        assert_eq!(parse_pair::<i32>("10,", ','), None);
        assert_eq!(parse_pair::<i32>(",10", ','), None);
        assert_eq!(parse_pair::<i32>("10,20", ','), Some((10, 20)));
        assert_eq!(parse_pair::<i32>("10,20xy", ','), None);
        assert_eq!(parse_pair::<f64>("0.5x", ','), None);
        assert_eq!(parse_pair::<f64>("0.5x1.5", 'x'), Some((0.5, 1.5)));
    }

    #[test]
    fn test_parse_complex() {
        assert_eq!(parse_complex("1.25,-0.0625"),
                   Some(Complex { re: 1.25, im: -0.0625 }));
        assert_eq!(parse_complex(",-0.0625"), None);
    }

    #[bench]
    fn bench_escapes(b: &mut Bencher) {
        let upper_left = Complex { re: -1.20, im: 0.35 };

        b.iter(|| escapes(upper_left, 255));
    }

    #[bench]
    fn bench_render_100_by_100(b: &mut Bencher) {
        let bounds = (100, 100);
        let upper_left = Complex { re: -1.20, im: 0.35 };
        let lower_right = Complex { re: -1.0, im: 0.20 };
        let mut pixels = vec![0; bounds.0 * bounds.1];

        b.iter(|| render(&mut pixels, bounds, upper_left, lower_right));
    }

    #[bench]
    fn bench_render_threaded_1000_by_1000(b: &mut Bencher) {
        let bounds = (1000, 1000);
        let upper_left = Complex { re: -1.20, im: 0.35 };
        let lower_right = Complex { re: -1.0, im: 0.20 };
        let mut pixels = vec![0; bounds.0 * bounds.1];

        b.iter(|| render(&mut pixels, bounds, upper_left, lower_right));
    }

    // TODO fix this test
    #[test]
    #[ignore]
    fn compare_gold_masters() {
        let tmp_dir = TempDir::new("output_tests").expect("create temp dir failed");
        println!("Generating test files in {:?}", tmp_dir);
        let filename = tmp_dir.path().join("mandel_4000x3000.png");
        let bounds = (4000, 3000);
        let upper_left = Complex { re: -1.20, im: 0.35 };
        let lower_right = Complex { re: -1.0, im: 0.20 };
        let mut pixels = vec![0; bounds.0 * bounds.1];

        render(&mut pixels, bounds, upper_left, lower_right);
        write_bitmap(&filename, &pixels, bounds).expect("error writing PNG file");
        println!("Written bitmap to '{:?}'", filename);

        // Compare output to the "golden master" file generated in the first version
        assert!(!dir_diff::is_different(&tmp_dir.path(), "gold_masters").unwrap());
    }
}