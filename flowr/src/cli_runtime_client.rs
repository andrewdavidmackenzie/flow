use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;

use image::{ImageBuffer, ImageFormat, Rgb, RgbImage};
use log::{debug, info};

use flowrlib::client_server::RuntimeClientConnection;
use flowrlib::runtime::{Event, Response};

#[derive(Debug, Clone)]
pub struct CLIRuntimeClient {
    args: Vec<String>,
    image_buffers: HashMap<String, ImageBuffer<Rgb<u8>, Vec<u8>>>,
    display_metrics: bool,
}

impl CLIRuntimeClient {
    fn new(args: Vec<String>, display_metrics: bool) -> Self {
        CLIRuntimeClient {
            args,
            image_buffers: HashMap::<String, ImageBuffer<Rgb<u8>, Vec<u8>>>::new(),
            display_metrics,
        }
    }

    /*
        Enter  a loop where we receive events as a client and respond to them
     */
    pub fn start(connection: RuntimeClientConnection,
                 flow_args: Vec<String>,
                 #[cfg(feature = "metrics")]
                 display_metrics: bool,
    ) {
        Self::capture_control_c(&connection);

        let mut runtime_client = CLIRuntimeClient::new(flow_args, display_metrics);

        loop {
            match connection.client_recv() {
                Ok(event) => {
                    let response = runtime_client.process_event(event);
                    if response == Response::ClientExiting {
                        return;
                    }
                    let _ = connection.client_send(response);
                }
                Err(_) => {
                    return;
                }
            }
        }
    }

    fn capture_control_c(_connection: &RuntimeClientConnection) {
        // let connection_clone = connection.clone();
        // let _ = ctrlc::set_handler(move || {
        //     let _ = connection_clone.send(Response::EnterDebugger);
        // });
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
                    image_buffer.save_with_format(Path::new(&filename), ImageFormat::Png).unwrap();
                }
                Response::ClientExiting
            }
            #[cfg(not(feature = "metrics"))]
            Event::FlowEnd => {
                debug!("=========================== Flow execution ended ======================================");
                for (filename, image_buffer) in self.image_buffers.drain() {
                    info!("Flushing ImageBuffer to file: {}", filename);
                    image_buffer.save_with_format(Path::new(&filename), ImageFormat::Png).unwrap();
                }
                Response::ClientExiting
            }
            Event::StdoutEOF => Response::Ack,
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
                        Response::GetStdinEOF
                    };
                }
                Response::Error("Could not read Stdin".into())
            }
            Event::GetLine => {
                let mut input = String::new();
                match io::stdin().read_line(&mut input) {
                    Ok(n) if n > 0 => Response::Line(input.trim().to_string()),
                    Ok(n) if n == 0 => Response::GetLineEOF,
                    _ => Response::Error("Could not read Readline".into())
                }
            }
            Event::Write(filename, bytes) => {
                let mut file = File::create(filename).unwrap();
                file.write_all(bytes.as_slice()).unwrap();
                Response::Ack
            }
            Event::PixelWrite((x, y), (r, g, b), (width, height), name) => {
                let image = self.image_buffers.entry(name)
                    .or_insert_with(|| RgbImage::new(width, height));
                image.put_pixel(x, y, Rgb([r, g, b]));
                Response::Ack
            }
            Event::GetArgs => {
                Response::Args(self.args.clone())
            }
            Event::StderrEOF => Response::Ack
        }
    }
}

#[cfg(test)]
mod test {
    use std::fs;

    use tempdir::TempDir;

    use flowrlib::metrics::Metrics;
    use flowrlib::runtime::{Event, Response};

    use super::CLIRuntimeClient;

    #[test]
    fn test_arg_passing() {
        let mut client = CLIRuntimeClient::new(vec!("file:///test_flow.toml".to_string(), "1".to_string()),
                                               false);

        match client.process_event(Event::GetArgs) {
            Response::Args(args) => assert_eq!(vec!("file:///test_flow.toml".to_string(), "1".to_string()), args),
            _ => panic!("Didn't get Args response as expected")
        }
    }

    #[test]
    fn test_file_writing() {
        let temp = tempdir::TempDir::new("flow").unwrap().into_path();
        let file = temp.join("test");

        let mut client = CLIRuntimeClient::new(vec!("file:///test_flow.toml".to_string()),
                                               false);

        if client.process_event(Event::Write(file.to_str().unwrap().to_string(), b"Hello".to_vec()))
            != Response::Ack {
            panic!("Didn't get Write response as expected")
        }
    }

    #[test]
    fn test_stdout() {
        let mut client = CLIRuntimeClient::new(vec!("file:///test_flow.toml".to_string()),
                                               false);
        if client.process_event(Event::Stdout("Hello".into())) != Response::Ack {
            panic!("Didn't get Stdout response as expected")
        }
    }

    #[test]
    fn test_stderr() {
        let mut client = CLIRuntimeClient::new(vec!("file:///test_flow.toml".to_string()),
                                               false);
        if client.process_event(Event::Stderr("Hello".into())) != Response::Ack {
            panic!("Didn't get Stderr response as expected")
        }
    }

    #[test]
    fn test_image_writing() {
        let mut client = CLIRuntimeClient::new(vec!("file:///test_flow.toml".to_string()),
                                               false);

        let temp_dir = TempDir::new("flow").unwrap().into_path();
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
}