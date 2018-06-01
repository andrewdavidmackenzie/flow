use std::fs::File;
use std::path::PathBuf;
use std::io;
use flowrlib::implementation::Implementation;
use flowrlib::implementation::RunAgain;
use flowrlib::runlist::RunList;
use flowrlib::runnable::Runnable;
use image::png::PNGEncoder;
use image::ColorType;
use serde_json::Value as JsonValue;

pub struct WriteBitmap;

impl Implementation for WriteBitmap {
    fn run(&self, _runnable: &Runnable, mut inputs: Vec<Vec<JsonValue>>, _run_list: &mut RunList) -> RunAgain {
        let filename = inputs.remove(0).remove(0);
        let bytes = inputs.remove(0).remove(0);
        let bounds = inputs.remove(0).remove(0);

        // bounds: (usize, usize),
        let width = bounds["width"].as_u64().unwrap() as usize;
        let height = bounds["height"].as_u64().unwrap() as usize;

        debug!("Writing image of width '{}' and height '{}' to file: '{}'",
            width, height, filename);
        write_bitmap(&PathBuf::from(filename.as_str().unwrap()),
                     bytes.as_str().unwrap().as_bytes(),
                     (width, height)).unwrap();

        true
    }
}

/// Write the buffer 'pixels', whose dimensions are given by 'bounds', to the file named 'filename'
fn write_bitmap(filename: &PathBuf, pixels: &[u8], bounds: (usize, usize)) -> io::Result<()> {
    let output = File::create(filename).unwrap();

    let encoder = PNGEncoder::new(output);
    encoder.encode(&pixels, bounds.0 as u32, bounds.1 as u32, ColorType::Gray(8))
}