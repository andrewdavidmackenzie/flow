use std::sync::{Arc, Mutex};

use serde_json::Value;

use flow_impl::{Implementation, RUN_AGAIN, RunAgain};

use crate::client_server::RuntimeServerContext;
use crate::runtime::{Event, Response};

/// `Implementation` struct for the `image_buffer` function
pub struct ImageBuffer {
    /// It holds a reference to the runtime client in order to send commands
    pub server_context: Arc<Mutex<RuntimeServerContext>>
}

impl Implementation for ImageBuffer {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        let pixel = inputs[0].as_array().unwrap();
        let value = inputs[1].as_array().unwrap();
        let size = inputs[2].as_array().unwrap();
        if let Value::String(filename) = &inputs[3] {
            if let Ok(mut server) = self.server_context.lock() {
                return match server.send_event(Event::PixelWrite(
                    (pixel[0].as_u64().unwrap() as u32, pixel[1].as_u64().unwrap() as u32),
                    (value[0].as_u64().unwrap() as u8, value[1].as_u64().unwrap() as u8, value[2].as_u64().unwrap() as u8),
                    (size[0].as_u64().unwrap() as u32, size[1].as_u64().unwrap() as u32),
                    filename.to_string()
                )) {
                    Response::Ack => (None, RUN_AGAIN),
                    _ => (None, RUN_AGAIN)
                }
            }
        }

        (None, RUN_AGAIN)
    }
}