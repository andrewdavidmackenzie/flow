//! Image buffer context function.

use std::sync::{Arc, Mutex};

use flowcore::errors::Result;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};
use serde_json::Value;

use crate::coordinator::client_message::ClientMessage;
use crate::coordinator::coordinator_connection::CoordinatorConnection;
use crate::coordinator::coordinator_message::CoordinatorMessage;

/// Image buffer context function for pixel writes
pub struct ImageBuffer {
    pub server_connection: Arc<Mutex<CoordinatorConnection>>,
}

impl Implementation for ImageBuffer {
    #[allow(clippy::many_single_char_names)]
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let pixel = inputs
            .first()
            .ok_or("Could not get pixels")?
            .as_array()
            .ok_or("Could not get pixels")?;
        let value = inputs
            .get(1)
            .ok_or("Could not get value")?
            .as_array()
            .ok_or("Could not get value")?;
        let size = inputs
            .get(2)
            .ok_or("Could not get size")?
            .as_array()
            .ok_or("Could not get size")?;
        let filename = inputs
            .get(3)
            .ok_or("Could not get filename")?
            .as_str()
            .ok_or("Could not get filename")?;

        let mut server = self
            .server_connection
            .lock()
            .map_err(|_| "Could not lock server")?;

        let x = pixel
            .first()
            .ok_or("Could not get x")?
            .as_u64()
            .ok_or("Could not get x")?;
        let y = pixel
            .get(1)
            .ok_or("Could not get y")?
            .as_u64()
            .ok_or("Could not get y")?;
        let r = value
            .first()
            .ok_or("Could not get r")?
            .as_u64()
            .ok_or("Could not get r")?;
        let g = value
            .get(1)
            .ok_or("Could not get g")?
            .as_u64()
            .ok_or("Could not get g")?;
        let b = value
            .get(2)
            .ok_or("Could not get b")?
            .as_u64()
            .ok_or("Could not get b")?;
        let w = size
            .first()
            .ok_or("Could not get w")?
            .as_u64()
            .ok_or("Could not get w")?;
        let h = size
            .get(1)
            .ok_or("Could not get h")?
            .as_u64()
            .ok_or("Could not get h")?;

        let _: Result<ClientMessage> =
            server.send_and_receive_response(CoordinatorMessage::PixelWrite(
                (
                    u32::try_from(x).map_err(|_| "Integer overflow in 'x'")?,
                    u32::try_from(y).map_err(|_| "Integer overflow in 'y'")?,
                ),
                (
                    u8::try_from(r).map_err(|_| "Integer overflow in 'r'")?,
                    u8::try_from(g).map_err(|_| "Integer overflow in 'g'")?,
                    u8::try_from(b).map_err(|_| "Integer overflow in 'b'")?,
                ),
                (
                    u32::try_from(w).map_err(|_| "Integer overflow in 'w'")?,
                    u32::try_from(h).map_err(|_| "Integer overflow in 'h'")?,
                ),
                filename.to_string(),
            ));

        Ok((None, RUN_AGAIN))
    }
}
