use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;

use image::{ImageBuffer, ImageFormat, Rgb, RgbImage};
use log::{debug, info};

use flowrlib::runtime_client::{Command, Response, RuntimeClient};

#[derive(Debug, Clone)]
pub struct CLIRuntimeClient {
    image_buffers: HashMap<String, ImageBuffer<Rgb<u8>, Vec<u8>>>
}

/// The name of the environment variables used to pass command line arguments to the function
/// used to get them.
pub const FLOW_ARGS_NAME: &str = "FLOW_ARGS";

impl CLIRuntimeClient {
    pub fn new() -> Self {
        CLIRuntimeClient {
            image_buffers: HashMap::<String, ImageBuffer<Rgb<u8>, Vec<u8>>>::new()
        }
    }

    #[allow(clippy::many_single_char_names)]
    fn process_command(&mut self, command: Command) -> Response {
        match command {
            Command::FlowStart => {
                debug!("===========================    Starting flow execution =============================");
                Response::Ack
            },
            Command::FlowEnd => {
                debug!("=========================== Flow execution ended ======================================");

                for (filename, image_buffer) in self.image_buffers.iter() {
                    info!("Flushing ImageBuffer to file: {}", filename);
                    image_buffer.save_with_format(Path::new(filename), ImageFormat::Png).unwrap();
                }

                Response::Ack
            },
            Command::EOF => Response::Ack,
            Command::Stdout(contents) => {
                println!("{}", contents);
                Response::Ack
            }
            Command::Stderr(contents) => {
                eprintln!("{}", contents);
                Response::Ack
            }
            Command::Stdin => {
                let mut buffer = String::new();
                let stdin = io::stdin();
                let mut handle = stdin.lock();
                if let Ok(size) = handle.read_to_string(&mut buffer) {
                    return if size > 0 {
                        Response::Stdin(buffer.trim().to_string())
                    } else {
                        Response::EOF
                    };
                }
                Response::Error("Could not read Stdin".into())
            }
            Command::Readline => {
                let mut input = String::new();
                match io::stdin().read_line(&mut input) {
                    Ok(n) if n > 0 => Response::Readline(input.trim().to_string()),
                    Ok(n) if n == 0 => Response::EOF,
                    _ => Response::Error("Could not read Readline".into())
                }
            }
            Command::Write(filename, bytes) => {
                let mut file = File::create(filename).unwrap();
                file.write_all(bytes.as_slice()).unwrap();
                Response::Ack
            }
            Command::PixelWrite((x, y), (r, g, b), (width, height), name) => {
                let image = self.image_buffers.entry(name)
                    .or_insert_with(|| RgbImage::new(width, height));
                image.put_pixel(x, y, Rgb([r, g, b]));
                Response::Ack
            }
            Command::Args => {
                let args = env::var(FLOW_ARGS_NAME).unwrap();
                env::remove_var(FLOW_ARGS_NAME); // so another invocation later won't use it by mistake
                let flow_args: Vec<String> = args.split(' ').map(|s| s.to_string()).collect();
                Response::Args(flow_args)
            }
        }
    }
}

impl RuntimeClient for CLIRuntimeClient {
    // This function is called by the runtime_function to send a command to the runtime_client
    // so here in the runtime_client, it's more like "process_command"
    fn send_command(&mut self, command: Command) -> Response {
        self.process_command(command)
    }
}

#[cfg(test)]
mod test {
    use std::env;
    use std::fs;

    use tempdir::TempDir;

    use flowrlib::runtime_client::{Command, Response, RuntimeClient};

    use super::CLIRuntimeClient;
    use super::FLOW_ARGS_NAME;

    #[test]
    fn test_arg_passing() {
        env::set_var(FLOW_ARGS_NAME, "test");

        let mut client = CLIRuntimeClient::new();

        match client.send_command(Command::Args) {
            Response::Args(args) => assert_eq!(vec!("test".to_string()), args),
            _ => panic!("Didn't get Args response as expected")
        }
    }

    #[test]
    fn test_file_writing() {
        let temp = tempdir::TempDir::new("flow").unwrap().into_path();
        let file = temp.join("test");

        let mut client = CLIRuntimeClient::new();

        if client.send_command(Command::Write(file.to_str().unwrap().to_string(), b"Hello".to_vec()))
            != Response::Ack {
            panic!("Didn't get Write response as expected")
        }
    }

    #[test]
    fn test_stdout() {
        let mut client = CLIRuntimeClient::new();
        if client.send_command(Command::Stdout("Hello".into())) != Response::Ack {
            panic!("Didn't get Stdout response as expected")
        }
    }

    #[test]
    fn test_stderr() {
        let mut client = CLIRuntimeClient::new();
        if client.send_command(Command::Stderr("Hello".into())) != Response::Ack {
            panic!("Didn't get Stderr response as expected")
        }
    }

    #[test]
    fn test_image_writing() {
        let mut client = CLIRuntimeClient::new();

        let temp_dir = TempDir::new("flow").unwrap().into_path();
        let path = temp_dir.join("flow.png");

        let _ = fs::remove_file(&path);
        assert!(!path.exists());

        client.send_command(Command::FlowStart);
        let pixel = Command::PixelWrite((0, 0), (255, 200, 20), (10, 10), path.display().to_string());
        if client.send_command(pixel) != Response::Ack {
            panic!("Didn't get pixel write response as expected")
        }
        client.send_command(Command::FlowEnd);

        assert!(path.exists(), "Image file was not created");
    }
}