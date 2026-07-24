use flowcore::errors::Result;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};
use serde_json::Value;

use crate::cli::coordinator_message::CoordinatorMessage;
use crate::context::ContextIO;

/// `Implementation` struct for the `image_buffer` function
pub struct ImageBuffer {
    pub context_io: ContextIO,
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

        self.context_io
            .send_and_receive(CoordinatorMessage::PixelWrite(
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
            ))?;

        Ok((None, RUN_AGAIN))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use flowcore::{Implementation, RUN_AGAIN};
    use serde_json::json;

    use crate::cli::coordinator_message::{ClientMessage, CoordinatorMessage};
    use crate::context::ContextIO;

    use super::ImageBuffer;

    fn make_image_buffer() -> (
        ImageBuffer,
        std::sync::mpsc::Receiver<crate::context::ContextRequest>,
    ) {
        let (tx, rx) = std::sync::mpsc::channel();
        let (blocking_tx, _blocking_rx) = std::sync::mpsc::channel();
        (
            ImageBuffer {
                context_io: ContextIO::new(tx, blocking_tx),
            },
            rx,
        )
    }

    #[test]
    fn invalid_parameters() {
        let (buffer, _rx) = make_image_buffer();
        let pixel = (0, 0);
        let color = (1, 2, 3);
        let invalid_size = (1.2, 3.4); // invalid
        let buffer_name: String = "image_buffer.png".into();
        let inputs = [
            json!(pixel),
            json!(color),
            json!(invalid_size),
            json!(buffer_name),
        ];
        assert!(buffer.run(&inputs).is_err());
    }

    #[test]
    fn valid() {
        let (buffer, rx) = make_image_buffer();
        let pixel = (0, 0);
        let color = (1, 2, 3);
        let size = (1, 3);
        let buffer_name: String = "image_buffer.png".into();
        let inputs = [json!(pixel), json!(color), json!(size), json!(buffer_name)];

        let handle = std::thread::spawn(move || buffer.run(&inputs));

        let req = rx.recv().expect("No request received");
        assert!(matches!(req.message, CoordinatorMessage::PixelWrite(..)));
        if let Some(response_tx) = req.response_tx {
            response_tx
                .send(ClientMessage::Ack)
                .expect("Could not send response");
        }

        let (value, run_again) = handle.join().unwrap().expect("run() failed");
        assert_eq!(run_again, RUN_AGAIN);
        assert_eq!(value, None);
    }
}
