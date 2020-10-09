use std::io::Write;

use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use image::ColorType;
use image::png::PngEncoder;
use serde_json::Value;

#[derive(FlowImpl)]
/// Format a series of bytes into a PNG image, for use in display or writing to a file
#[derive(Debug)]
pub struct FormatPNG;

impl Implementation for FormatPNG {
    fn run(&self, inputs: &[Value])
           -> (Option<Value>, RunAgain) {
        let bytes = &inputs[0];

        // bounds: (usize, usize),
        let bounds = &inputs[1];
        let width = bounds["width"].as_u64().unwrap() as u32;
        let height = bounds["height"].as_u64().unwrap() as u32;

//        debug!("Writing image of width '{}' and height '{}'", width, height);

        let mut png_buffer = Vec::new();
        let encoder = PngEncoder::new(png_buffer.by_ref());
        match encoder.encode(bytes.as_str().unwrap().as_bytes(), width, height, ColorType::L8)
        {
            Ok(_) => {}
            Err(e) => println!("Error '{}' while encoding bytes as PNG", e)
        }

        // TODO
//        let string = String::from_utf8_lossy(&png_buffer).to_string();
//        run_list.send_output(function, Value::String(string));
        (None, RUN_AGAIN)
    }
}
