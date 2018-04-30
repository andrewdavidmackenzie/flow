#![feature(test)]

extern crate num;
extern crate image;
extern crate rayon;
extern crate test;
extern crate dir_diff;
extern crate tempdir;
extern crate flowrlib;
#[macro_use] extern crate serde_json;

mod parse_args;
mod escapes;
mod pixel_to_point;
mod create_complex;
mod parse_pair;
mod render_by_row;
mod render_a_row;
mod render_pixel;

use render_by_row::render_by_row;
use parse_args::parse_args;
use num::Complex;
use std::fs::File;
use std::path::PathBuf;
use std::io::Result;
use image::ColorType;
use image::png::PNGEncoder;

fn main() {
    let (filename, bounds, upper_left, lower_right) = parse_args();

    let mut pixels = vec![0; bounds.0 * bounds.1];

    render_by_row(&mut pixels, bounds, upper_left, lower_right);

    write_bitmap(&filename, &pixels, bounds).expect("error writing PNG file");
}

/// Write the buffer 'pixels', whose dimensions are given by 'bounds', to the file named 'filename'
fn write_bitmap(filename: &PathBuf, pixels: &[u8], bounds: (usize, usize)) -> Result<()> {
    let output = File::create(filename)?;

    let encoder = PNGEncoder::new(output);
    encoder.encode(&pixels, bounds.0 as u32, bounds.1 as u32,
                   ColorType::Gray(8))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;
    use test::Bencher;

    #[bench]
    fn bench_render_100_by_100(b: &mut Bencher) {
        let bounds = (100, 100);
        let upper_left = Complex { re: -1.20, im: 0.35 };
        let lower_right = Complex { re: -1.0, im: 0.20 };
        let mut pixels = vec![0; bounds.0 * bounds.1];

        b.iter(|| render_by_row(&mut pixels, bounds, upper_left, lower_right));
    }

    #[bench]
    fn bench_render_1000_by_1000(b: &mut Bencher) {
        let bounds = (1000, 1000);
        let upper_left = Complex { re: -1.20, im: 0.35 };
        let lower_right = Complex { re: -1.0, im: 0.20 };
        let mut pixels = vec![0; bounds.0 * bounds.1];

        b.iter(|| render_by_row(&mut pixels, bounds, upper_left, lower_right));
    }

    #[test]
    fn compare_gold_masters() {
        let tmp_dir = TempDir::new("output_tests").expect("create temp dir failed");
        println!("Generating test files in {:?}", tmp_dir);
        let filename = tmp_dir.path().join("mandel_4000x3000.png");
        let bounds = (4000, 3000);
        let upper_left = Complex { re: -1.20, im: 0.35 };
        let lower_right = Complex { re: -1.0, im: 0.20 };
        let mut pixels = vec![0; bounds.0 * bounds.1];

        render_by_row(&mut pixels, bounds, upper_left, lower_right);
        write_bitmap(&filename, &pixels, bounds).expect("error writing PNG file");
        println!("Written bitmap to '{:?}'", filename);

        // Compare output to the "golden master" file generated previously
        assert!(!dir_diff::is_different(&tmp_dir.path(), "gold_masters").unwrap());
    }
}