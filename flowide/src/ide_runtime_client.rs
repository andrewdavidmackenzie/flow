use std::fs::File;
use std::io::prelude::*;

use gtk::TextBufferExt;

use flowruntime::runtime_client::{Command, Response, RuntimeClient};

use crate::widgets;

#[derive(Debug)]
pub struct IDERuntimeClient {
    args: Vec<String>
}

impl IDERuntimeClient {
    pub fn new() -> Self {
        IDERuntimeClient{args: vec!()}
    }

    pub fn set_args(&mut self, args: Vec<String>) {
        self.args = args;
    }
}

impl RuntimeClient for IDERuntimeClient {
    fn init(&self) {}

    // This function is called by the runtime_function to send a commanmd to the runtime_client
    fn send_command(&self, command: Command) -> Response {
        match command {
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
        }
    }
}