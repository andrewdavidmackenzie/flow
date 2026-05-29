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
                let pixels: Vec<Value> = gray.into_raw().into_iter().map(|v| json!(v)).collect();

                let mut output_map = serde_json::Map::new();
                output_map.insert("pixels".into(), Value::Array(pixels));
                output_map.insert("width".into(), json!(width));
                output_map.insert("height".into(), json!(height));

                Ok((Some(Value::Object(output_map)), DONT_RUN_AGAIN))
            }
            _ => Ok((None, DONT_RUN_AGAIN)),
        }
    }
}
