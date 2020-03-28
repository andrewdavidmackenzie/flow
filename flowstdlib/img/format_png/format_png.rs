use std::io::Write;

use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use image::ColorType;
use image::png::PNGEncoder;
use serde_json::Value;

#[derive(FlowImpl)]
/// Format a series of bytes into a PNG image, for use in display or writing to a file
///
/// ## Include using
/// ```toml
/// [[process]]
/// alias = "format"
/// source = "lib://flowstdlib/img/format_png"
/// ```
///
/// ## Input
/// * `bytes` - the bytes to encode as a bitmap
/// * `bounds` - Json with width and depth of the image the bytes represent
///
/// ## Output
/// * (default) - The bytes representing the PNG encoding of the image, as a String
#[derive(Debug)]
pub struct FormatPNG;

impl Implementation for FormatPNG {
    fn run(&self, inputs: &Vec<Vec<Value>>)
           -> (Option<Value>, RunAgain) {
        let bytes = &inputs[0][0];

        // bounds: (usize, usize),
        let bounds = &inputs[1][0];
        let width = bounds["width"].as_u64().unwrap() as u32;
        let height = bounds["height"].as_u64().unwrap() as u32;

//        debug!("Writing image of width '{}' and height '{}'", width, height);

        let mut png_buffer = Vec::new();
        let encoder = PNGEncoder::new(png_buffer.by_ref());
        match encoder.encode(bytes.as_str().unwrap().as_bytes(), width, height, ColorType::Gray(8))
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
