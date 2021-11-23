use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;

use image::{ImageBuffer, ImageFormat, Rgb, RgbImage};
use log::{debug, error, info};

use flowrlib::client_server::ClientConnection;
use flowrlib::errors::*;
use flowrlib::runtime_messages::{ClientMessage, FileMetaData, ServerMessage};

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
        #[cfg(feature = "debugger")] control_c_connection: Option<
            ClientConnection<'static, ServerMessage, ClientMessage>,
        >,
        connection: ClientConnection<ServerMessage, ClientMessage>,
    ) -> Result<()> {
        #[cfg(feature = "debugger")]
        if let Some(control_c) = control_c_connection {
            Self::enter_debugger_on_control_c(control_c);
        }

        loop {
            match connection.receive() {
                Ok(event) => {
                    let response = self.process_server_message(event);
                    if response == ClientMessage::ClientExiting {
                        debug!("Client has decided to exit, so exiting the event loop.");
                        return Ok(());
                    }

                    let _ = connection.send(response);
                }
                Err(e) => {
                    // When debugging, a Control-C to break into the debugger will cause receive()
                    // to return an error. Ignore it so we continue to process events from server
                    error!("Error receiving message from server: '{}'", e);
                }
            }
        }
    }

    #[cfg(feature = "debugger")]
    fn enter_debugger_on_control_c(
        #[cfg(feature = "debugger")] control_c_connection: ClientConnection<'static, ServerMessage, ClientMessage>,
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

    fn flush_image_buffers(&mut self) {
        for (filename, image_buffer) in self.image_buffers.drain() {
            info!("Flushing ImageBuffer to file: {}", filename);
            if let Err(e) = image_buffer.save_with_format(Path::new(&filename), ImageFormat::Png) {
                error!("Error saving ImageBuffer '{}': '{}'", filename, e);
            }
        }
    }

    #[allow(clippy::many_single_char_names)]
    pub fn process_server_message(&mut self, message: ServerMessage) -> ClientMessage {
        match message {
            #[cfg(feature = "metrics")]
            ServerMessage::FlowEnd(metrics) => {
                debug!("=========================== Flow execution ended ======================================");
                if self.display_metrics {
                    println!("\nMetrics: \n {}", metrics);
                }

                self.flush_image_buffers();
                ClientMessage::ClientExiting
            }

            #[cfg(not(feature = "metrics"))]
            ServerMessage::FlowEnd => {
                debug!("=========================== Flow execution ended ======================================");
                self.flush_image_buffers();
                ClientMessage::ClientExiting
            }
            ServerMessage::FlowStart => {
                debug!("===========================    Starting flow execution =============================");
                ClientMessage::Ack
            }
            ServerMessage::ServerExiting => {
                debug!("Server is exiting");
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
            ServerMessage::Read(file_path) => match File::open(&file_path) {
                Ok(mut f) => {
                    let mut buffer = Vec::new();
                    match f.read_to_end(&mut buffer) {
                        Ok(_) => ClientMessage::FileContents(file_path, buffer),
                        Err(_) => ClientMessage::Error(format!(
                            "Could not read content from '{:?}'",
                            file_path
                        )),
                    }
                }
                Err(_) => ClientMessage::Error(format!("Could not open file '{:?}'", file_path)),
            },
            ServerMessage::GetFileMetaData(path) => match std::fs::metadata(&path) {
                Ok(md) => ClientMessage::FileMetaDate(
                    path,
                    FileMetaData {
                        is_file: md.is_file(),
                        is_dir: md.is_dir(),
                    },
                ),
                Err(_) => {
                    ClientMessage::Error(format!("Could not read file metadata from '{:?}'", path))
                }
            },
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
            ServerMessage::GetArgs => ClientMessage::Args(self.args.clone()),
            ServerMessage::StderrEof => ClientMessage::Ack,
            ServerMessage::Invalid => ClientMessage::Ack,
        }
    }
}

#[cfg(test)]
mod test {
    use std::fs;
    use std::fs::File;
    use std::io::prelude::*;

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

        match client.process_server_message(ServerMessage::GetArgs) {
            ClientMessage::Args(args) => assert_eq!(
                vec!("file:///test_flow.toml".to_string(), "1".to_string()),
                args
            ),
            _ => panic!("Didn't get Args response as expected"),
        }
    }

    #[test]
    fn test_file_reading() {
        let test_contents = b"The quick brown fox jumped over the lazy dog";

        let temp = tempdir::TempDir::new("flow")
            .expect("Couldn't get TempDir")
            .into_path();
        let file_path = temp.join("test_read");
        {
            let mut file = File::create(&file_path).expect("Could not create test file");
            file.write_all(test_contents)
                .expect("Could not write to test file");
        }
        let mut client = CliRuntimeClient::new(
            vec!["file:///test_flow.toml".to_string()],
            #[cfg(feature = "metrics")]
            false,
        );

        match client.process_server_message(ServerMessage::Read(file_path.clone())) {
            ClientMessage::FileContents(path_read, contents) => {
                assert_eq!(path_read, file_path);
                assert_eq!(contents, test_contents)
            }
            _ => panic!("Didn't get Write response as expected"),
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

        if client.process_server_message(ServerMessage::Write(
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
        if client.process_server_message(ServerMessage::Stdout("Hello".into()))
            != ClientMessage::Ack
        {
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
        if client.process_server_message(ServerMessage::Stderr("Hello".into()))
            != ClientMessage::Ack
        {
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

        client.process_server_message(ServerMessage::FlowStart);
        let pixel =
            ServerMessage::PixelWrite((0, 0), (255, 200, 20), (10, 10), path.display().to_string());
        if client.process_server_message(pixel) != ClientMessage::Ack {
            panic!("Didn't get pixel write response as expected")
        }

        #[cfg(not(feature = "metrics"))]
        client.process_server_message(ServerMessage::FlowEnd);
        #[cfg(feature = "metrics")]
        client.process_server_message(ServerMessage::FlowEnd(Metrics::new(1)));

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
            client.process_server_message(ServerMessage::ServerExiting),
            ClientMessage::ClientExiting
        );
    }
}
