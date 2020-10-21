use std::fs::File;
use std::io::prelude::*;

use gtk::TextBufferExt;

use flowrlib::runtime_client::{Command, Response, RuntimeClient};

use crate::widgets;

#[derive(Debug)]
pub struct IDERuntimeClient {
    args: Vec<String>
}

impl IDERuntimeClient {
    pub fn new() -> Self {
        IDERuntimeClient{args: vec!()}
    }

    fn process_command(&mut self, command: Command) -> Response {
        match command {
            Command::FlowStart => Response::Ack,
            Command::FlowEnd => Response::Ack,
            Command::EOF => Response::Ack,
            Command::Stdout(contents) => {
                widgets::do_in_gtk_eventloop(|refs| {
                    refs.stdout().insert_at_cursor(&format!("{}\n", contents));
                });
                Response::Ack
            }
            Command::Stderr(contents) => {
                widgets::do_in_gtk_eventloop(|refs| {
                    refs.stderr().insert_at_cursor(&format!("{}\n", contents));
                });
                Response::Ack
            }
            Command::Stdin => {
//                Response::Stdin("bla bla".to_string()) // TODO
                Response::Error("Could not read Stdin".into())
            }
            Command::Readline => {
                Response::Stdin("bla bla".to_string())  // TODO
            }
            Command::Args => {
                Response::Args(self.args.clone())
            }
            Command::Write(filename, bytes) => {
                let mut file = File::create(filename).unwrap();
                file.write_all(bytes.as_slice()).unwrap();
                Response::Ack
            }
            Command::PixelWrite((_x, _y), (_r, _g, _b), (_width, _height), _name) => {
                // let image = self.image_buffers.entry(name)
                //     .or_insert(RgbImage::new(width, height));
                // image.put_pixel(x, y, Rgb([r, g, b]));
                Response::Ack
            }
        }
    }

    pub fn set_args(&mut self, args: Vec<String>) {
        self.args = args;
    }
}

impl RuntimeClient for IDERuntimeClient {
    // This function is called by the runtime_function to send a command to the runtime_client
    fn send_command(&mut self, command: Command) -> Response {
        self.process_command(command)
    }
}