use std::sync::{Arc, Mutex};

use flowcore::errors::Result;
use flowcore::{Implementation, RunAgain, DONT_RUN_AGAIN};
use image::ImageReader;
use serde_json::{json, Value};
use std::io::Cursor;

use crate::gui::coordinator_connection::CoordinatorConnection;
use crate::gui::coordinator_message::CoordinatorMessage;

/// `Implementation` struct for the `image_read` function
pub struct ImageRead {
    /// It holds a reference to the runtime client in order to read files
    pub server_connection: Arc<Mutex<CoordinatorConnection>>,
}

impl Implementation for ImageRead {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let filename = inputs
            .first()
            .ok_or("Could not get filename")?
            .as_str()
            .ok_or("Could not get filename as string")?;

        let mut server = self
            .server_connection
            .lock()
            .map_err(|_| "Could not lock server")?;

        let response =
            server.send_and_receive_response(CoordinatorMessage::Read(filename.to_string()));

        match response {
            Ok(crate::gui::client_message::ClientMessage::FileContents(_path, bytes)) => {
                let img = ImageReader::new(Cursor::new(bytes))
                    .with_guessed_format()
                    .map_err(|e| format!("Could not guess image format: {e}"))?
                    .decode()
                    .map_err(|e| format!("Could not decode image: {e}"))?;

                let gray = img.to_luma8();
                let width = gray.width();
                let height = gray.height();

                let raw_pixels: Vec<Value> = gray.as_raw().iter().map(|&v| json!(v)).collect();
                let mut pixels_eof: Vec<Value> = raw_pixels.clone();
                pixels_eof.push(Value::Null);

                let grid: Vec<Value> = (0..height)
                    .map(|y| {
                        let row: Vec<Value> = (0..width)
                            .map(|x| json!(gray.get_pixel(x, y).0[0]))
                            .collect();
                        Value::Array(row)
                    })
                    .collect();

                let raw = gray.as_raw();
                let pixel_coords: Vec<Value> = (0..height)
                    .flat_map(|y| {
                        (0..width).map(move |x| {
                            let idx = (y * width + x) as usize;
                            let v = raw.get(idx).copied().unwrap_or(0);
                            json!([x, y, v])
                        })
                    })
                    .collect();

                let mut output_map = serde_json::Map::new();
                output_map.insert("pixels".into(), Value::Array(raw_pixels));
                output_map.insert("pixels_eof".into(), Value::Array(pixels_eof));
                output_map.insert("grid".into(), Value::Array(grid));
                output_map.insert("pixel_coords".into(), Value::Array(pixel_coords));
                output_map.insert("width".into(), json!(width));
                output_map.insert("height".into(), json!(height));
                output_map.insert("size".into(), json!([width, height]));

                Ok((Some(Value::Object(output_map)), DONT_RUN_AGAIN))
            }
            _ => Ok((None, DONT_RUN_AGAIN)),
        }
    }
}
