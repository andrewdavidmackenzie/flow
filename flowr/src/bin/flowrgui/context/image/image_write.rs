use flowcore::errors::Result;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};
use serde_json::Value;

use crate::context::ContextIO;
use crate::gui::coordinator_message::CoordinatorMessage;

/// `Implementation` struct for the `image_write` function
pub struct ImageWrite {
    /// It holds a reference to the runtime client in order to send commands
    pub context_io: ContextIO,
}

fn to_u8(v: &Value) -> u8 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let result = v.as_f64().unwrap_or(0.0).clamp(0.0, 255.0) as u8;
    result
}

impl Implementation for ImageWrite {
    fn run(&self, inputs: &[Value]) -> Result<(Option<Value>, RunAgain)> {
        let grid_input = inputs
            .first()
            .ok_or("Could not get grid")?
            .as_array()
            .ok_or("Could not get grid as array")?;
        let filename = inputs
            .get(1)
            .ok_or("Could not get filename")?
            .as_str()
            .ok_or("Could not get filename as string")?;

        let grid: Vec<Vec<u8>> = if grid_input.first().is_some_and(|v| v.as_array().is_some()) {
            grid_input
                .iter()
                .map(|row| {
                    row.as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .map(to_u8)
                        .collect()
                })
                .collect()
        } else {
            let width = inputs
                .get(2)
                .and_then(Value::as_u64)
                .and_then(|w| usize::try_from(w).ok())
                .filter(|&w| w > 0)
                .unwrap_or(1);
            let flat: Vec<u8> = grid_input.iter().map(to_u8).collect();
            flat.chunks(width).map(<[u8]>::to_vec).collect()
        };

        self.context_io
            .send_and_receive(CoordinatorMessage::ImageWrite(grid, filename.to_string()))?;

        Ok((None, RUN_AGAIN))
    }
}
