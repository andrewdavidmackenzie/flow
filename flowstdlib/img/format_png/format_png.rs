extern crate core;
extern crate flow_impl;
extern crate flow_impl_derive;
extern crate serde_json;

use flow_impl::{Implementation, RUN_AGAIN, RunAgain};
use flow_impl_derive::FlowImpl;
use image::ColorType;
use image::png::PNGEncoder;
use serde_json::Value;
use std::io::Write;

#[derive(FlowImpl)]
pub struct FormatPNG;

impl Implementation for FormatPNG {
    fn run(&self, mut inputs: Vec<Vec<Value>>)
        -> (Option<Value>, RunAgain) {
        let bytes = inputs.remove(0).remove(0);

        // bounds: (usize, usize),
        let bounds = inputs.remove(0).remove(0);
        let width = bounds["width"].as_u64().unwrap() as u32;
        let height = bounds["height"].as_u64().unwrap() as u32;

//        debug!("Writing image of width '{}' and height '{}'", width, height);

        let mut png_buffer = Vec::new();
        let encoder = PNGEncoder::new(png_buffer.by_ref());
        encoder.encode(bytes.as_str().unwrap().as_bytes(), width, height, ColorType::Gray(8))
            .expect("error encoding pixels as PNG");


        // TODO
//        let string = String::from_utf8_lossy(&png_buffer).to_string();
//        run_list.send_output(function, Value::String(string));
        (None, RUN_AGAIN)
    }
}
