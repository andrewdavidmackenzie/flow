use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;

use image::{ImageBuffer, ImageFormat, Rgb, RgbImage};
use log::{debug, trace};

use flowrlib::runtime_client::{Command, Response, RuntimeClient};

#[derive(Debug, Clone)]
pub struct CLIRuntimeClient {
    pub image_buffers: HashMap<String, ImageBuffer<Rgb<u8>, Vec<u8>>>
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
}

impl RuntimeClient for CLIRuntimeClient {
    fn flow_start(&mut self) {
        debug!("===========================    Starting flow execution =============================");
    }

    // This function is called by the runtime_function to send a command to the runtime_client
    // so here in the runtime_client, it's more like "process_command"
    #[allow(clippy::many_single_char_names)]
    fn send_command(&mut self, command: Command) -> Response {
        match command {
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

    fn flow_end(&mut self) {
        debug!("=========================== Flow execution ended ======================================");

        // flush ImageBuffers to disk
        for (filename, image_buffer) in self.image_buffers.iter() {
            trace!("Flushing ImageBuffer to file: {}", filename);
            image_buffer.save_with_format(Path::new(filename), ImageFormat::PNG).unwrap();
        }
    }
}

#[cfg(test)]
mod test {
    use std::env;
    use std::fs;
    use std::path::Path;

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
        let path = "/tmp/flow.png";

        fs::remove_file(path).unwrap();
        assert!(!Path::new(path).exists());

        client.flow_start();
        let pixel = Command::PixelWrite((0, 0), (255, 200, 20), (10, 10), path.into());
        if client.send_command(pixel) != Response::Ack {
            panic!("Didn't get pixel write response as expected")
        }
        client.flow_end();

        assert!(Path::new(path).exists());
    }
}