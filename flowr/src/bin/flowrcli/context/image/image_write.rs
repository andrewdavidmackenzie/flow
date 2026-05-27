use std::sync::{Arc, Mutex};

use flowcore::errors::Result;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};
use serde_json::Value;

use crate::cli::connections::CoordinatorConnection;
use crate::cli::coordinator_message::{ClientMessage, CoordinatorMessage};

/// `Implementation` struct for the `image_write` function
pub struct ImageWrite {
    /// It holds a reference to the runtime client in order to send commands
    pub server_connection: Arc<Mutex<CoordinatorConnection>>,
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

        let grid: Vec<Vec<u8>> = grid_input
            .iter()
            .map(|row| {
                row.as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|v| u8::try_from(v.as_u64().unwrap_or(0)).unwrap_or(0))
                    .collect()
            })
            .collect();

        let mut server = self
            .server_connection
            .lock()
            .map_err(|_| "Could not lock server")?;

        let _: Result<ClientMessage> = server
            .send_and_receive_response(CoordinatorMessage::ImageWrite(grid, filename.to_string()));

        Ok((None, RUN_AGAIN))
    }
}
