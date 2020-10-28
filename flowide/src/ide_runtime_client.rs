use std::fs::File;
use std::io::prelude::*;

use gtk::TextBufferExt;

use flowrlib::client_server::RuntimeClient;
use flowrlib::runtime::{Event, Response};

use crate::widgets;

#[derive(Debug)]
pub struct IDERuntimeClient {
    args: Vec<String>
}

impl IDERuntimeClient {
    pub fn new() -> Self {
        IDERuntimeClient{args: vec!()}
    }

    fn process_command(&mut self, command: Event) -> Response {
        match command {
            Event::FlowStart => Response::Ack,
            Event::FlowEnd(_) => Response::Ack,
            Event::StdoutEOF => Response::Ack,
            Event::Stdout(contents) => {
                widgets::do_in_gtk_eventloop(|refs| {
                    refs.stdout().insert_at_cursor(&format!("{}\n", contents));
                });
                Response::Ack
            }
            Event::Stderr(contents) => {
                widgets::do_in_gtk_eventloop(|refs| {
                    refs.stderr().insert_at_cursor(&format!("{}\n", contents));
                });
                Response::Ack
            }
            Event::GetStdin => {
//                Response::Stdin("bla bla".to_string()) // TODO
                Response::Error("Could not read Stdin".into())
            }
            Event::GetLine => {
                Response::Stdin("bla bla".to_string())  // TODO
            }
            Event::GetArgs => {
                Response::Args(self.args.clone())
            }
            Event::Write(filename, bytes) => {
                let mut file = File::create(filename).unwrap();
                file.write_all(bytes.as_slice()).unwrap();
                Response::Ack
            }
            Event::PixelWrite((_x, _y), (_r, _g, _b), (_width, _height), _name) => {
                // let image = self.image_buffers.entry(name)
                //     .or_insert(RgbImage::new(width, height));
                // image.put_pixel(x, y, Rgb([r, g, b]));
                Response::Ack
            }
            Event::StderrEOF => Response::Ack
        }
    }

    pub fn set_args(&mut self, args: Vec<String>) {
        self.args = args;
    }
}

impl RuntimeClient for IDERuntimeClient {
    // This function is called by the runtime_function to send a command to the runtime_client
    fn send_event(&mut self, command: Event) -> Response {
        self.process_command(command)
    }
}