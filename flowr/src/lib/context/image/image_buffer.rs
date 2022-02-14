use std::sync::{Arc, Mutex};

use serde_json::Value;

use flowcore::{Implementation, RUN_AGAIN, RunAgain};
use flowcore::errors::*;

use crate::client_server::ServerConnection;
use crate::runtime_messages::{ClientMessage, ServerMessage};

/// `Implementation` struct for the `image_buffer` function
pub struct ImageBuffer {
    /// It holds a reference to the runtime client in order to send commands
    pub server_connection: Arc<Mutex<ServerConnection>>,
}

impl Implementation for ImageBuffer {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let pixel = inputs[0].as_array().ok_or("Could not get pixel")?;
        let value = inputs[1].as_array().ok_or("Could not get value")?;
        let size = inputs[2].as_array().ok_or("Could not get size")?;
        let filename = &inputs[3].as_str().ok_or("Could not get filename")?;

        let mut server = self.server_connection.lock()
            .map_err(|_| "Could not lock server")?;

        let x = pixel[0].as_u64().ok_or("Could not get x")?;
        let y = pixel[1].as_u64().ok_or("Could not get y")?;
        let r = value[0].as_u64().ok_or("Could not get r")?;
        let g = value[1].as_u64().ok_or("Could not get g")?;
        let b = value[2].as_u64().ok_or("Could not get b")?;
        let w = size[0].as_u64().ok_or("Could not get w")?;
        let h = size[1].as_u64().ok_or("Could not get h")?;

        let _: crate::errors::Result<ClientMessage> = server.send_and_receive_response(ServerMessage::PixelWrite(
                (x as u32, y as u32),
                (r as u8, g as u8, b as u8),
                (w as u32, h as u32),
                filename.to_string(),
            ));

        Ok((None, RUN_AGAIN))
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;
    use serial_test::serial;

    use flowcore::{Implementation, RUN_AGAIN};

    use crate::runtime_messages::{ClientMessage, ServerMessage};

    use super::ImageBuffer;
    use super::super::super::test_helper::test::wait_for_then_send;

    #[test]
    #[serial]
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
        assert!(buffer.run(&inputs).is_err());
    }

    #[test]
    #[serial]
    fn valid() {
        let pixel = (0, 0);
        let color = (1, 2, 3);
        let size = (1, 3);
        let buffer_name = "image_buffer.png".into();
        let inputs = [json!(pixel), json!(color), json!(size), json!(buffer_name)];
        let pixel = ServerMessage::PixelWrite(pixel, color, size, buffer_name);

        let server_connection = wait_for_then_send(pixel, ClientMessage::Ack);
        let buffer = &ImageBuffer { server_connection } as &dyn Implementation;
        let (value, run_again) = buffer.run(&inputs).expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }
}
