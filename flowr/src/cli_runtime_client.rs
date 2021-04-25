use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;

use image::{ImageBuffer, ImageFormat, Rgb, RgbImage};
use log::{debug, error, info, trace};

use flowrlib::client_server::RuntimeClientConnection;
use flowrlib::coordinator::Submission;
use flowrlib::errors::*;
use flowrlib::runtime::Response::ClientSubmission;
use flowrlib::runtime::{Event, Response};

#[derive(Debug, Clone)]
pub struct CliRuntimeClient {
    args: Vec<String>,
    image_buffers: HashMap<String, ImageBuffer<Rgb<u8>, Vec<u8>>>,
    #[cfg(feature = "metrics")]
    display_metrics: bool,
}

impl CliRuntimeClient {
    fn new(args: Vec<String>, #[cfg(feature = "metrics")] display_metrics: bool) -> Self {
        CliRuntimeClient {
            args,
            image_buffers: HashMap::<String, ImageBuffer<Rgb<u8>, Vec<u8>>>::new(),
            #[cfg(feature = "metrics")]
            display_metrics,
        }
    }

    /*
       Enter  a loop where we receive events as a client and respond to them
    */
    pub fn start(
        mut connection: RuntimeClientConnection,
        submission: Submission,
        flow_args: Vec<String>,
        #[cfg(feature = "metrics")] display_metrics: bool,
    ) -> Result<()> {
        connection.start()?;
        trace!("Connection from Runtime client to Runtime server started");

        debug!("Client sending submission to server");
        connection.client_send(ClientSubmission(submission))?;

        let mut runtime_client = CliRuntimeClient::new(
            flow_args,
            #[cfg(feature = "metrics")]
            display_metrics,
        );

        loop {
            debug!("Client waiting for message from server");
            match connection.client_recv() {
                Ok(event) => {
                    trace!("Runtime client received event from server: {:?}", event);
                    let response = runtime_client.process_event(event);
                    if response == Response::ClientExiting {
                        debug!("Server is exiting, so client will exit also");
                        return Ok(());
                    }

                    trace!("Runtime client sending response to server: {:?}", response);
                    let _ = connection.client_send(response);
                }
                Err(e) => {
                    bail!("Error receiving Event in runtime client: {}", e);
                }
            }
        }
    }

    #[allow(clippy::many_single_char_names)]
    pub fn process_event(&mut self, event: Event) -> Response {
        match event {
            Event::FlowStart => {
                debug!("===========================    Starting flow execution =============================");
                Response::Ack
            }
            #[cfg(feature = "metrics")]
            Event::FlowEnd(metrics) => {
                debug!("=========================== Flow execution ended ======================================");
                info!("\nMetrics: \n {}", metrics);

                for (filename, image_buffer) in self.image_buffers.drain() {
                    info!("Flushing ImageBuffer to file: {}", filename);
                    if let Err(e) =
                        image_buffer.save_with_format(Path::new(&filename), ImageFormat::Png)
                    {
                        error!("Error saving ImageBuffer '{}': '{}'", filename, e);
                    }
                }
                Response::ClientExiting
            }
            Event::ServerExiting => {
                debug!("Server is exiting");
                Response::ClientExiting
            }
            #[cfg(not(feature = "metrics"))]
            Event::FlowEnd => {
                debug!("=========================== Flow execution ended ======================================");
                for (filename, image_buffer) in self.image_buffers.drain() {
                    info!("Flushing ImageBuffer to file: {}", filename);
                    if let Err(e) =
                        image_buffer.save_with_format(Path::new(&filename), ImageFormat::Png)
                    {
                        error!("Error saving ImageBuffer '{}': '{}'", filename, e);
                    }
                }
                Response::ClientExiting
            }
            Event::StdoutEof => Response::Ack,
            Event::Stdout(contents) => {
                println!("{}", contents);
                Response::Ack
            }
            Event::Stderr(contents) => {
                eprintln!("{}", contents);
                Response::Ack
            }
            Event::GetStdin => {
                let mut buffer = String::new();
                let stdin = io::stdin();
                let mut handle = stdin.lock();
                if let Ok(size) = handle.read_to_string(&mut buffer) {
                    return if size > 0 {
                        Response::Stdin(buffer.trim().to_string())
                    } else {
                        Response::GetStdinEof
                    };
                }
                Response::Error("Could not read Stdin".into())
            }
            Event::GetLine => {
                let mut input = String::new();
                match io::stdin().read_line(&mut input) {
                    Ok(n) if n > 0 => Response::Line(input.trim().to_string()),
                    Ok(n) if n == 0 => Response::GetLineEof,
                    _ => Response::Error("Could not read Readline".into()),
                }
            }
            Event::Write(filename, bytes) => match File::create(&filename) {
                Ok(mut file) => match file.write_all(bytes.as_slice()) {
                    Ok(_) => Response::Ack,
                    Err(e) => {
                        let msg = format!("Error writing to file: '{}': '{}'", filename, e);
                        error!("{}", msg);
                        Response::Error(msg)
                    }
                },
                Err(e) => {
                    let msg = format!("Error creating file: '{}': '{}'", filename, e);
                    error!("{}", msg);
                    Response::Error(msg)
                }
            },
            Event::PixelWrite((x, y), (r, g, b), (width, height), name) => {
                let image = self
                    .image_buffers
                    .entry(name)
                    .or_insert_with(|| RgbImage::new(width, height));
                image.put_pixel(x, y, Rgb([r, g, b]));
                Response::Ack
            }
            Event::GetArgs => {
                // Response gets serialized and sent over channel/network so needs to args be owned
                Response::Args(self.args.clone())
            }
            Event::StderrEof => Response::Ack,
            Event::Invalid => Response::Ack,
        }
    }
}

#[cfg(test)]
mod test {
    use std::fs;

    use tempdir::TempDir;

    #[cfg(feature = "metrics")]
    use flowrlib::metrics::Metrics;
    use flowrlib::runtime::{Event, Response};

    use super::CliRuntimeClient;

    #[test]
    fn test_arg_passing() {
        let mut client = CliRuntimeClient::new(
            vec!["file:///test_flow.toml".to_string(), "1".to_string()],
            #[cfg(feature = "metrics")]
            false,
        );

        match client.process_event(Event::GetArgs) {
            Response::Args(args) => assert_eq!(
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

        if client.process_event(Event::Write(
            file.to_str().expect("Couldn't get filename").to_string(),
            b"Hello".to_vec(),
        )) != Response::Ack
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
        if client.process_event(Event::Stdout("Hello".into())) != Response::Ack {
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
        if client.process_event(Event::Stderr("Hello".into())) != Response::Ack {
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

        client.process_event(Event::FlowStart);
        let pixel = Event::PixelWrite((0, 0), (255, 200, 20), (10, 10), path.display().to_string());
        if client.process_event(pixel) != Response::Ack {
            panic!("Didn't get pixel write response as expected")
        }
        #[cfg(feature = "metrics")]
        client.process_event(Event::FlowEnd(Metrics::new(1)));
        #[cfg(not(feature = "metrics"))]
        client.process_event(Event::FlowEnd);

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
            client.process_event(Event::ServerExiting),
            Response::ClientExiting
        );
    }
}
