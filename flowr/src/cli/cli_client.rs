use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::sync::{Arc, Mutex};

use image::{ImageBuffer, ImageFormat, Rgb, RgbImage};
use log::debug;
#[cfg(feature = "debugger")]
use log::error;
#[cfg(feature = "debugger")]
use log::info;

use flowcore::errors::*;

use crate::cli::connections::ClientConnection;
use crate::cli::coordinator_message::{ClientMessage, CoordinatorMessage};

#[derive(Debug, Clone)]
pub struct CliRuntimeClient {
    args: Vec<String>,
    override_args: Arc<Mutex<Vec<String>>>,
    image_buffers: HashMap<String, ImageBuffer<Rgb<u8>, Vec<u8>>>,
    #[cfg(feature = "metrics")] display_metrics: bool,
}

impl CliRuntimeClient {
    /// Create a new runtime client
    pub fn new(args: Vec<String>,
               override_args: Arc<Mutex<Vec<String>>>,
               #[cfg(feature = "metrics")] display_metrics: bool) -> Self {
        CliRuntimeClient {
            args,
            override_args,
            image_buffers: HashMap::<String, ImageBuffer<Rgb<u8>, Vec<u8>>>::new(),
            #[cfg(feature = "metrics")] display_metrics,
        }
    }

    /// Enter a loop where we receive events as a client and respond to them
    pub fn event_loop(
        mut self,
        connection: ClientConnection,
    ) -> Result<()> {
        loop {
            match connection.receive() {
                Ok(event) => {
                    let response = self.process_coordinator_message(event);
                    if let ClientMessage::ClientExiting(coordinator_result) = response {
                        debug!("Client is exiting the event loop.");
                        return coordinator_result;
                    }

                    let _ = connection.send(response);
                }
                Err(e) => {
                    // When debugging, a Control-C to break into the debugger will cause receive()
                    // to return an error. Ignore it so we continue to process events from coordinator
                    bail!("Error receiving message from coordinator: '{}'", e);
                }
            }
        }
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
            #[cfg(feature = "metrics")]
            CoordinatorMessage::FlowEnd(metrics) => {
                debug!("=========================== Flow execution ended ======================================");
                if self.display_metrics {
                    println!("\nMetrics: \n {metrics}");
                    let _ = io::stdout().flush();
                }

                self.flush_image_buffers();
                ClientMessage::ClientExiting(Ok(()))
            }

            #[cfg(not(feature = "metrics"))]
            CoordinatorMessage::FlowEnd => {
                debug!("=========================== Flow execution ended ======================================");
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

#[cfg(test)]
mod test {
    use std::fs;
    use std::fs::File;
    use std::io::prelude::*;
    use std::sync::{Arc, Mutex};

    use tempdir::TempDir;

    #[cfg(feature = "metrics")]
    use flowcore::model::metrics::Metrics;

    use crate::cli::coordinator_message::{ClientMessage, CoordinatorMessage};

    use super::CliRuntimeClient;

    #[test]
    fn test_arg_passing() {
        let mut client = CliRuntimeClient::new(
            vec!["file:///test_flow.toml".to_string(), "1".to_string()],
            Arc::new(Mutex::new(vec!())),
            #[cfg(feature = "metrics")]
            false,
        );

        match client.process_coordinator_message(CoordinatorMessage::GetArgs) {
            ClientMessage::Args(args) => assert_eq!(
                vec!("file:///test_flow.toml".to_string(), "1".to_string()),
                args
            ),
            _ => panic!("Didn't get Args response as expected"),
        }
    }

    #[test]
    fn test_arg_overriding() {
        let override_args = Arc::new(Mutex::new(vec!()));
        let mut client = CliRuntimeClient::new(
            vec!["file:///test_flow.toml".to_string(), "1".to_string()],
            override_args.clone(),
            #[cfg(feature = "metrics")]
                false,
        );

        {
            let mut overrides = override_args.lock()
                .expect("Could not lock override args");
            overrides.push("override".into());
        }

        match client.process_coordinator_message(CoordinatorMessage::GetArgs) {
            ClientMessage::Args(args) => assert_eq!(
                vec!("file:///test_flow.toml".to_string(), "override".to_string()),
                args
            ),
            _ => panic!("Args override response was not as expected"),
        }
    }

    #[test]
    fn test_file_reading() {
        let test_contents = b"The quick brown fox jumped over the lazy dog";

        let temp = TempDir::new("flow")
            .expect("Couldn't get TempDir")
            .into_path();
        let file_path = temp.join("test_read").to_string_lossy().to_string();
        {
            let mut file = File::create(&file_path).expect("Could not create test file");
            file.write_all(test_contents)
                .expect("Could not write to test file");
        }
        let mut client = CliRuntimeClient::new(
            vec!["file:///test_flow.toml".to_string()],
            Arc::new(Mutex::new(vec!())),
            #[cfg(feature = "metrics")]
            false,
        );

        match client.process_coordinator_message(CoordinatorMessage::Read(file_path.clone())) {
            ClientMessage::FileContents(path_read, contents) => {
                assert_eq!(path_read, file_path);
                assert_eq!(contents, test_contents)
            }
            _ => panic!("Didn't get Read response as expected"),
        }
    }

    #[test]
    fn test_file_writing() {
        let temp = TempDir::new("flow")
            .expect("Couldn't get TempDir")
            .into_path();
        let file = temp.join("test");

        let mut client = CliRuntimeClient::new(
            vec!["file:///test_flow.toml".to_string()],
            Arc::new(Mutex::new(vec!())),
            #[cfg(feature = "metrics")]
            false,
        );

        match client.process_coordinator_message(CoordinatorMessage::Write(
            file.to_str().expect("Couldn't get filename").to_string(),
            b"Hello".to_vec())) {
            ClientMessage::Ack => {},
            _ => panic!("Didn't get Write response as expected"),
        }
    }

    #[test]
    fn test_stdout() {
        let mut client = CliRuntimeClient::new(
            vec!["file:///test_flow.toml".to_string()],
            Arc::new(Mutex::new(vec!())),
            #[cfg(feature = "metrics")]
            false,
        );
        match client.process_coordinator_message(CoordinatorMessage::Stdout("Hello".into())) {
            ClientMessage::Ack => {},
            _ => panic!("Didn't get Stdout response as expected"),
        }
    }

    #[test]
    fn test_stderr() {
        let mut client = CliRuntimeClient::new(
            vec!["file:///test_flow.toml".to_string()],
            Arc::new(Mutex::new(vec!())),
            #[cfg(feature = "metrics")]
            false,
        );
        match client.process_coordinator_message(CoordinatorMessage::Stderr("Hello".into())) {
            ClientMessage::Ack => {},
            _ => panic!("Didn't get Stderr response as expected"),
        }
    }

    #[test]
    fn test_image_writing() {
        let mut client = CliRuntimeClient::new(
            vec!["file:///test_flow.toml".to_string()],
            Arc::new(Mutex::new(vec!())),
            #[cfg(feature = "metrics")]
            false,
        );

        let temp_dir = TempDir::new("flow")
            .expect("Couldn't get TempDir")
            .into_path();
        let path = temp_dir.join("flow.png");

        let _ = fs::remove_file(&path);
        assert!(!path.exists());

        client.process_coordinator_message(CoordinatorMessage::FlowStart);
        let pixel =
            CoordinatorMessage::PixelWrite((0, 0), (255, 200, 20), (10, 10), path.display().to_string());
        match client.process_coordinator_message(pixel) {
            ClientMessage::Ack => {},
            _ => panic!("Didn't get pixel write response as expected"),
        }

        #[cfg(not(feature = "metrics"))]
        client.process_coordinator_message(CoordinatorMessage::FlowEnd);
        #[cfg(feature = "metrics")]
        client.process_coordinator_message(CoordinatorMessage::FlowEnd(Metrics::new(1)));

        assert!(path.exists(), "Image file was not created");
    }

    #[test]
    fn coordinator_exiting() {
        let mut client = CliRuntimeClient::new(
            vec!["file:///test_flow.toml".to_string()],
            Arc::new(Mutex::new(vec!())),
            #[cfg(feature = "metrics")] false,
        );

        match client.process_coordinator_message(CoordinatorMessage::CoordinatorExiting(Ok(()))) {
            ClientMessage::ClientExiting(_) => {},
            _ => panic!("Didn't get ClientExiting response as expected"),
        }
    }
}
