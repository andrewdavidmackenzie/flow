use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;

use image::{ImageBuffer, ImageFormat, Rgb, RgbImage};
use log::{debug, error, info};

use flowrlib::client_server::ClientConnection;
use flowrlib::coordinator::Submission;
use flowrlib::errors::*;
use flowrlib::runtime_messages::ClientMessage::ClientSubmission;
use flowrlib::runtime_messages::{ClientMessage, ServerMessage};

#[derive(Debug, Clone)]
pub struct CliRuntimeClient {
    args: Vec<String>,
    image_buffers: HashMap<String, ImageBuffer<Rgb<u8>, Vec<u8>>>,
    #[cfg(feature = "metrics")]
    display_metrics: bool,
}

impl CliRuntimeClient {
    /// Create a new runtime client
    pub fn new(args: Vec<String>, #[cfg(feature = "metrics")] display_metrics: bool) -> Self {
        CliRuntimeClient {
            args,
            image_buffers: HashMap::<String, ImageBuffer<Rgb<u8>, Vec<u8>>>::new(),
            #[cfg(feature = "metrics")]
            display_metrics,
        }
    }

    /// Enter a loop where we receive events as a client and respond to them
    pub fn event_loop(
        mut self,
        connection: ClientConnection<ServerMessage, ClientMessage>,
        #[cfg(feature = "debugger")] control_c_connection: ClientConnection<
            'static,
            ServerMessage,
            ClientMessage,
        >,
        submission: Submission,
        debugger: bool,
    ) -> Result<()> {
        #[cfg(feature = "debugger")]
        if debugger {
            Self::enter_debugger_on_control_c(control_c_connection);
        }

        debug!("Client sending submission to server");
        connection.send(ClientSubmission(submission))?;

        loop {
            match connection.receive() {
                Ok(event) => {
                    let response = self.process_event(event);
                    if response == ClientMessage::ClientExiting {
                        debug!("Client has decided to exit, so exiting the event loop.");
                        return Ok(());
                    }

                    let _ = connection.send(response);
                }
                Err(e) => {
                    // When debugging a Control-C to break into the debugger will cause receive()
                    // to return an error. Ignore it so we continue to process events from server
                    if !debugger {
                        bail!("Error receiving Event in runtime client: {}", e);
                    }
                }
            }
        }
    }

    #[cfg(feature = "debugger")]
    fn enter_debugger_on_control_c(
        control_c_connection: ClientConnection<'static, ServerMessage, ClientMessage>,
    ) {
        ctrlc::set_handler(move || {
            info!("Control-C captured in client.");
            match control_c_connection.send(ClientMessage::EnterDebugger) {
                Ok(_) => debug!("'EnterDebugger' command sent to Server"),
                Err(e) => error!(
                    "Error sending 'EnterDebugger' command to server on control_c_connection: {}",
                    e
                ),
            }
        })
        .expect("Error setting Ctrl-C handler");
    }

    #[allow(clippy::many_single_char_names)]
    pub fn process_event(&mut self, event: ServerMessage) -> ClientMessage {
        match event {
            ServerMessage::FlowStart => {
                debug!("===========================    Starting flow execution =============================");
                ClientMessage::Ack
            }
            #[cfg(feature = "metrics")]
            ServerMessage::FlowEnd(metrics) => {
                debug!("=========================== Flow execution ended ======================================");
                if self.display_metrics {
                    println!("\nMetrics: \n {}", metrics);
                }

                for (filename, image_buffer) in self.image_buffers.drain() {
                    info!("Flushing ImageBuffer to file: {}", filename);
                    if let Err(e) =
                        image_buffer.save_with_format(Path::new(&filename), ImageFormat::Png)
                    {
                        error!("Error saving ImageBuffer '{}': '{}'", filename, e);
                    }
                }
                ClientMessage::ClientExiting
            }
            ServerMessage::ServerExiting => {
                debug!("Server is exiting");
                ClientMessage::ClientExiting
            }
            #[cfg(not(feature = "metrics"))]
            ServerMessage::FlowEnd => {
                debug!("=========================== Flow execution ended ======================================");
                for (filename, image_buffer) in self.image_buffers.drain() {
                    info!("Flushing ImageBuffer to file: {}", filename);
                    if let Err(e) =
                        image_buffer.save_with_format(Path::new(&filename), ImageFormat::Png)
                    {
                        error!("Error saving ImageBuffer '{}': '{}'", filename, e);
                    }
                }
                ClientMessage::ClientExiting
            }
            ServerMessage::StdoutEof => ClientMessage::Ack,
            ServerMessage::Stdout(contents) => {
                println!("{}", contents);
                ClientMessage::Ack
            }
            ServerMessage::Stderr(contents) => {
                eprintln!("{}", contents);
                ClientMessage::Ack
            }
            ServerMessage::GetStdin => {
                let mut buffer = String::new();
                let stdin = io::stdin();
                let mut handle = stdin.lock();
                if let Ok(size) = handle.read_to_string(&mut buffer) {
                    return if size > 0 {
                        ClientMessage::Stdin(buffer.trim().to_string())
                    } else {
                        ClientMessage::GetStdinEof
                    };
                }
                ClientMessage::Error("Could not read Stdin".into())
            }
            ServerMessage::GetLine => {
                let mut input = String::new();
                match io::stdin().read_line(&mut input) {
                    Ok(n) if n > 0 => ClientMessage::Line(input.trim().to_string()),
                    Ok(n) if n == 0 => ClientMessage::GetLineEof,
                    _ => ClientMessage::Error("Could not read Readline".into()),
                }
            }
            ServerMessage::Write(filename, bytes) => match File::create(&filename) {
                Ok(mut file) => match file.write_all(bytes.as_slice()) {
                    Ok(_) => ClientMessage::Ack,
                    Err(e) => {
                        let msg = format!("Error writing to file: '{}': '{}'", filename, e);
                        error!("{}", msg);
                        ClientMessage::Error(msg)
                    }
                },
                Err(e) => {
                    let msg = format!("Error creating file: '{}': '{}'", filename, e);
                    error!("{}", msg);
                    ClientMessage::Error(msg)
                }
            },
            ServerMessage::PixelWrite((x, y), (r, g, b), (width, height), name) => {
                let image = self
                    .image_buffers
                    .entry(name)
                    .or_insert_with(|| RgbImage::new(width, height));
                image.put_pixel(x, y, Rgb([r, g, b]));
                ClientMessage::Ack
            }
            ServerMessage::GetArgs => {
                // Response gets serialized and sent over channel/network so needs to args be owned
                ClientMessage::Args(self.args.clone())
            }
            ServerMessage::StderrEof => ClientMessage::Ack,
            ServerMessage::Invalid => ClientMessage::Ack,
        }
    }
}

#[cfg(test)]
mod test {
    use std::fs;

    use tempdir::TempDir;

    #[cfg(feature = "metrics")]
    use flowrlib::metrics::Metrics;
    use flowrlib::runtime_messages::{ClientMessage, ServerMessage};

    use super::CliRuntimeClient;

    #[test]
    fn test_arg_passing() {
        let mut client = CliRuntimeClient::new(
            vec!["file:///test_flow.toml".to_string(), "1".to_string()],
            #[cfg(feature = "metrics")]
            false,
        );

        match client.process_event(ServerMessage::GetArgs) {
            ClientMessage::Args(args) => assert_eq!(
                vec!("file:///test_flow.toml".to_string(), "1".to_string()),
                args
            ),
            _ => panic!("Didn't get Args response as expected"),
        }
    }

    #[test]
    fn test_file_writing() {
        let temp = tempdir::TempDir::new("flow")
            .expect("Couldn't get TempDir")
            .into_path();
        let file = temp.join("test");

        let mut client = CliRuntimeClient::new(
            vec!["file:///test_flow.toml".to_string()],
            #[cfg(feature = "metrics")]
            false,
        );

        if client.process_event(ServerMessage::Write(
            file.to_str().expect("Couldn't get filename").to_string(),
            b"Hello".to_vec(),
        )) != ClientMessage::Ack
        {
            panic!("Didn't get Write response as expected")
        }
    }

    #[test]
    fn test_stdout() {
        let mut client = CliRuntimeClient::new(
            vec!["file:///test_flow.toml".to_string()],
            #[cfg(feature = "metrics")]
            false,
        );
        if client.process_event(ServerMessage::Stdout("Hello".into())) != ClientMessage::Ack {
            panic!("Didn't get Stdout response as expected")
        }
    }

    #[test]
    fn test_stderr() {
        let mut client = CliRuntimeClient::new(
            vec!["file:///test_flow.toml".to_string()],
            #[cfg(feature = "metrics")]
            false,
        );
        if client.process_event(ServerMessage::Stderr("Hello".into())) != ClientMessage::Ack {
            panic!("Didn't get Stderr response as expected")
        }
    }

    #[test]
    fn test_image_writing() {
        let mut client = CliRuntimeClient::new(
            vec!["file:///test_flow.toml".to_string()],
            #[cfg(feature = "metrics")]
            false,
        );

        let temp_dir = TempDir::new("flow")
            .expect("Couldn't get TempDir")
            .into_path();
        let path = temp_dir.join("flow.png");

        let _ = fs::remove_file(&path);
        assert!(!path.exists());

        client.process_event(ServerMessage::FlowStart);
        let pixel =
            ServerMessage::PixelWrite((0, 0), (255, 200, 20), (10, 10), path.display().to_string());
        if client.process_event(pixel) != ClientMessage::Ack {
            panic!("Didn't get pixel write response as expected")
        }
        #[cfg(feature = "metrics")]
        client.process_event(ServerMessage::FlowEnd(Metrics::new(1)));
        #[cfg(not(feature = "metrics"))]
        client.process_event(ServerMessage::FlowEnd);

        assert!(path.exists(), "Image file was not created");
    }

    #[test]
    fn server_exiting() {
        let mut client = CliRuntimeClient::new(
            vec!["file:///test_flow.toml".to_string()],
            #[cfg(feature = "metrics")]
            false,
        );

        assert_eq!(
            client.process_event(ServerMessage::ServerExiting),
            ClientMessage::ClientExiting
        );
    }
}
