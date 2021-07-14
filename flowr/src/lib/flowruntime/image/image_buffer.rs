use std::sync::{Arc, Mutex};

use serde_json::Value;

use flowcore::{Implementation, RunAgain, RUN_AGAIN};

use crate::client_server::ServerConnection;
use crate::errors::*;
use crate::runtime_messages::{ClientMessage, ServerMessage};

/// `Implementation` struct for the `image_buffer` function
pub struct ImageBuffer {
    /// It holds a reference to the runtime client in order to send commands
    pub server_context: Arc<Mutex<ServerConnection>>,
}

impl Implementation for ImageBuffer {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        if let (Some(pixel), Some(value), Some(size), Value::String(filename), Ok(ref mut server)) = (
            inputs[0].as_array(),
            inputs[1].as_array(),
            inputs[2].as_array(),
            &inputs[3],
            self.server_context.lock(),
        ) {
            if let (Some(x), Some(y), Some(r), Some(g), Some(b), Some(w), Some(h)) = (
                pixel[0].as_u64(),
                pixel[1].as_u64(),
                value[0].as_u64(),
                value[1].as_u64(),
                value[2].as_u64(),
                size[0].as_u64(),
                size[1].as_u64(),
            ) {
                let _: Result<ClientMessage> =
                    server.send_message::<ServerMessage, ClientMessage>(ServerMessage::PixelWrite(
                        (x as u32, y as u32),
                        (r as u8, g as u8, b as u8),
                        (w as u32, h as u32),
                        filename.to_string(),
                    ));
            }
        }

        (None, RUN_AGAIN)
    }
}
