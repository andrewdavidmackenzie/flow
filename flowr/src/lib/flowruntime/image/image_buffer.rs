use std::sync::{Arc, Mutex};

use serde_json::Value;

use flowcore::{Implementation, RunAgain, RUN_AGAIN};

use crate::client_server::ServerConnection;
use crate::errors::*;
use crate::runtime_messages::{ClientMessage, ServerMessage};

/// `Implementation` struct for the `image_buffer` function
pub struct ImageBuffer {
    /// It holds a reference to the runtime client in order to send commands
    pub server_connection: Arc<Mutex<ServerConnection<ServerMessage, ClientMessage>>>,
}

impl Implementation for ImageBuffer {
    fn run(&self, inputs: &[Value]) -> (Option<Value>, RunAgain) {
        if inputs.len() == 4 {
            if let (
                Some(pixel),
                Some(value),
                Some(size),
                Value::String(filename),
                Ok(ref mut server),
            ) = (
                inputs[0].as_array(),
                inputs[1].as_array(),
                inputs[2].as_array(),
                &inputs[3],
                self.server_connection.lock(),
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
                        server.send_and_receive_response(ServerMessage::PixelWrite(
                            (x as u32, y as u32),
                            (r as u8, g as u8, b as u8),
                            (w as u32, h as u32),
                            filename.to_string(),
                        ));
                }
            }
        }

        (None, RUN_AGAIN)
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use serial_test::serial;

    use flowcore::{Implementation, RUN_AGAIN};

    use crate::runtime_messages::{ClientMessage, ServerMessage};

    use super::super::super::test_helper::test::wait_for_then_send;
    use super::ImageBuffer;

    #[test]
    #[serial(client_server)]
    fn missing_parameters() {
        let pixel = (0, 0);
        let inputs = [json!(pixel)]; // Missing
        let pixel = ServerMessage::PixelWrite(pixel, (0, 0, 0), (1, 1), "image_buffer.png".into());

        let server_connection = wait_for_then_send(pixel, ClientMessage::Ack);
        let buffer = &ImageBuffer { server_connection } as &dyn Implementation;
        let (value, run_again) = buffer.run(&inputs);
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    #[serial(client_server)]
    fn invalid_parameters() {
        let pixel = (0, 0);
        let color = (1, 2, 3);
        let invalid_size = (1.2, 3.4); // invalid
        let size = (1, 3);
        let buffer_name = "image_buffer.png".into();
        let inputs = [
            json!(pixel),
            json!(color),
            json!(invalid_size),
            json!(buffer_name),
        ];
        let pixel = ServerMessage::PixelWrite(pixel, color, size, buffer_name);

        let server_connection = wait_for_then_send(pixel, ClientMessage::Ack);
        let buffer = &ImageBuffer { server_connection } as &dyn Implementation;
        let (value, run_again) = buffer.run(&inputs);
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }

    #[test]
    #[serial(client_server)]
    fn valid() {
        let pixel = (0, 0);
        let color = (1, 2, 3);
        let size = (1, 3);
        let buffer_name = "image_buffer.png".into();
        let inputs = [json!(pixel), json!(color), json!(size), json!(buffer_name)];
        let pixel = ServerMessage::PixelWrite(pixel, color, size, buffer_name);

        let server_connection = wait_for_then_send(pixel, ClientMessage::Ack);
        let buffer = &ImageBuffer { server_connection } as &dyn Implementation;
        let (value, run_again) = buffer.run(&inputs);
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }
}
