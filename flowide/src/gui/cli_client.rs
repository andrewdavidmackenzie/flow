use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::sync::{Arc, Mutex};

use image::{ImageBuffer, ImageFormat, Rgb, RgbImage};
use log::debug;
use log::error;
use log::info;

use flowcore::errors::*;

use crate::gui::connections::ClientConnection;
use crate::gui::coordinator_message::{ClientMessage, CoordinatorMessage};

pub struct CliRuntimeClient {
    connection: ClientConnection,
    args: Vec<String>,
    override_args: Arc<Mutex<Vec<String>>>,
    image_buffers: HashMap<String, ImageBuffer<Rgb<u8>, Vec<u8>>>,
    display_metrics: bool,
}

impl CliRuntimeClient {
    /// Create a new runtime client
    pub fn new(connection: ClientConnection) -> Self {
        CliRuntimeClient {
            connection,
            args: Vec::default(),
            override_args: Arc::new(Mutex::new(vec!["".into()])),
            image_buffers: HashMap::<String, ImageBuffer<Rgb<u8>, Vec<u8>>>::new(),
            display_metrics: false
        }
    }

    /// return a clone (reference) to the override args
    pub fn override_args(&self) -> Arc<Mutex<Vec<String>>> {
        self.override_args.clone()
    }

    /// Set the args to pass to the flow
    pub fn set_args(&mut self, args: &[String]) {
        self.args = args.to_vec();
    }

    /// Set or unset the flag to display metric
    pub fn set_display_metrics(&mut self, display_metrics: bool) {
        self.display_metrics = display_metrics;
    }

    /// Enter a loop where we receive events as a client and respond to them
    pub fn event_loop(&mut self) -> Result<()> {
        loop {
            match self.connection.receive() {
                Ok(event) => {
                    let response = self.process_coordinator_message(event);
                    if let ClientMessage::ClientExiting(coordinator_result) = response {
                        debug!("Client is exiting the event loop.");
                        return coordinator_result;
                    }

                    let _ = self.connection.send(response);
                }
                Err(e) => {
                    // When debugging, a Control-C to break into the debugger will cause receive()
                    // to return an error. Ignore it so we continue to process events from coordinator
                    bail!("Error receiving message from coordinator: '{}'", e);
                }
            }
        }
    }

    /// Send a message
    pub fn send(&mut self, message: ClientMessage) -> Result<()> {
        self.connection.send(message)
    }

    fn flush_image_buffers(&mut self) {
        for (filename, image_buffer) in self.image_buffers.drain() {
            info!("Flushing ImageBuffer to file: {}", filename);
            if let Err(e) = image_buffer.save_with_format(Path::new(&filename), ImageFormat::Png) {
                error!("Error saving ImageBuffer '{}': '{}'", filename, e);
            }
        }
    }

    fn process_coordinator_message(&mut self, message: CoordinatorMessage) -> ClientMessage {
        match message {
            CoordinatorMessage::FlowEnd(metrics) => {
                debug!("=========================== Flow execution ended ======================================");
                if self.display_metrics {
                    println!("\nMetrics: \n {metrics}");
                    let _ = io::stdout().flush();
                }

                self.flush_image_buffers();
                ClientMessage::ClientExiting(Ok(()))
            }
            CoordinatorMessage::FlowStart => {
                debug!("===========================    Starting flow execution =============================");
                ClientMessage::Ack
            }
            CoordinatorMessage::CoordinatorExiting(result) => {
                debug!("Coordinator is exiting");
                ClientMessage::ClientExiting(result)
            }
            CoordinatorMessage::StdoutEof => ClientMessage::Ack,
            CoordinatorMessage::Stdout(contents) => {
                let stdout = io::stdout();
                let mut handle = stdout.lock();
                let _ = handle.write_all(format!("{contents}\n").as_bytes());
                let _ = io::stdout().flush();
                ClientMessage::Ack
            }
            CoordinatorMessage::StderrEof => ClientMessage::Ack,
            CoordinatorMessage::Stderr(contents) => {
                let stderr = io::stderr();
                let mut handle = stderr.lock();
                let _ = handle.write_all(format!("{contents}\n").as_bytes());
                let _ = io::stdout().flush();
                ClientMessage::Ack
            }
            CoordinatorMessage::GetStdin => {
                let mut buffer = String::new();
                if let Ok(size) = io::stdin().read_to_string(&mut buffer) {
                    return if size > 0 {
                        ClientMessage::Stdin(buffer.trim().to_string())
                    } else {
                        ClientMessage::GetStdinEof
                    };
                }
                ClientMessage::Error("Could not read Stdin".into())
            }
            CoordinatorMessage::GetLine(prompt) => {
                let mut input = String::new();
                if !prompt.is_empty() {
                    print!("{}", prompt);
                    let _ = io::stdout().flush();
                }
                let line = io::stdin().lock().read_line(&mut input);
                match line {
                    Ok(n) if n > 0 => ClientMessage::Line(input.trim().to_string()),
                    Ok(n) if n == 0 => ClientMessage::GetLineEof,
                    _ => ClientMessage::Error("Could not read Readline".into()),
                }
            }
            CoordinatorMessage::Read(file_path) => match File::open(&file_path) {
                Ok(mut f) => {
                    let mut buffer = Vec::new();
                    match f.read_to_end(&mut buffer) {
                        Ok(_) => ClientMessage::FileContents(file_path, buffer),
                        Err(_) => ClientMessage::Error(format!(
                            "Could not read content from '{file_path:?}'"
                        )),
                    }
                }
                Err(_) => ClientMessage::Error(format!("Could not open file '{file_path:?}'")),
            },
            CoordinatorMessage::Write(filename, bytes) => match File::create(&filename) {
                Ok(mut file) => match file.write_all(bytes.as_slice()) {
                    Ok(_) => ClientMessage::Ack,
                    Err(e) => {
                        let msg = format!("Error writing to file: '{filename}': '{e}'");
                        error!("{msg}");
                        ClientMessage::Error(msg)
                    }
                },
                Err(e) => {
                    let msg = format!("Error creating file: '{filename}': '{e}'");
                    error!("{msg}");
                    ClientMessage::Error(msg)
                }
            },
            CoordinatorMessage::PixelWrite((x, y), (r, g, b), (width, height), name) => {
                let image = self
                    .image_buffers
                    .entry(name)
                    .or_insert_with(|| RgbImage::new(width, height));
                image.put_pixel(x, y, Rgb([r, g, b]));
                ClientMessage::Ack
            },
            CoordinatorMessage::GetArgs => {
                if let Ok(override_args) = self.override_args.lock() {
                    if override_args.is_empty() {
                        ClientMessage::Args(self.args.clone())
                    } else {
                        // we want to retain arg[0] which is the flow name and replace  all others
                        // with the override args supplied
                        let mut one_time_args = vec!(self.args[0].clone());
                        one_time_args.append(&mut override_args.to_vec());
                        ClientMessage::Args(one_time_args)
                    }
                } else {
                    ClientMessage::Args(self.args.clone())
                }
            },
            CoordinatorMessage::Invalid => ClientMessage::Ack,
        }
    }
}
