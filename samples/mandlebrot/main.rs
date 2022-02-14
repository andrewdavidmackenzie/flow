#![feature(test)]

use image::ColorType;
use image::png::PngEncoder;
use num::Complex;
use rayon::prelude::*;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use crate::pixel_to_point::pixel_to_point::pixel_to_point;
use crate::escapes::escapes::escapes;

mod escapes;
mod pixel_to_point;
mod parse_pair;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 5 {
        writeln!(std::io::stderr(), "Usage:   {} FILE PIXELS UPPERLEFT LOWERRIGHT", args[0])
            .expect("Could not write");
        writeln!(std::io::stderr(), "Example: {} mandel.png 1000x750 -1.2,0.35 -1,0.20", args[0])
            .expect("Could not write");
        std::process::exit(1);
    }

    let _executable_name = &args[0];

    let filename = PathBuf::from(&args[1]);

    let bounds = parse_pair::parse_pair(&args[2], "x").expect("error parsing image dimensions");

    let upper_left_args: (f64, f64) = parse_pair::parse_pair(&args[3], ",").expect("error parsing upper left corner point");
    let upper_left = Complex { re: upper_left_args.0, im: upper_left_args.1};

    let lower_right_args = parse_pair::parse_pair(&args[4], ",").expect("error parsing lower rightcorner point");
    let lower_right = Complex{ re: lower_right_args.0, im: lower_right_args.1};

    let mut pixels = vec![0; bounds.0 * bounds.1 * 3];

    render(&mut pixels, bounds, upper_left, lower_right);

    write_bitmap(&filename, &pixels, bounds);
}

fn render(pixels: &mut [u8], bounds: (usize, usize),
          upper_left: Complex<f64>, lower_right: Complex<f64>) {
    let rows = pixels.par_chunks_mut(bounds.0 * 3);
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

        let value = escapes(point, 255) as u8;
        pixels[offset] = value;
        offset += 1; // move forward a byte in the pixel buffer
        pixels[offset] = value;
        offset += 1; // move forward a byte in the pixel buffer
        pixels[offset] = value;
        offset += 1; // move forward a byte in the pixel buffer
    }
}

/// Write the buffer 'pixels', whose dimensions are given by 'bounds', to the file named 'filename'
fn write_bitmap(filename: &PathBuf, pixels: &[u8], bounds: (usize, usize)) {
    let output = File::create(filename).expect("Could not create file");
    let encoder = PngEncoder::new(output);
    encoder.encode(&pixels, bounds.0 as u32, bounds.1 as u32, ColorType::Rgb8).expect("Could not encode bytes as PNG");
}

#[cfg(test)]
mod test {
    extern crate test;

    use test::Bencher;

    use super::*;

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
}