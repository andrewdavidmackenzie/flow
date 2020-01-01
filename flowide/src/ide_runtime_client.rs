use std::fs::File;
use std::io::prelude::*;

use gtk::TextBufferExt;

use runtime::runtime_client::{Command, Response, RuntimeClient};

use crate::widgets;

pub struct IDERuntimeClient;

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
//                let (start, end) = runtime_context.args.get_bounds();
//                let arg_string = runtime_context.args.get_text(&start, &end, false).unwrap().to_string();
//                let args: Vec<String> = arg_string.split(' ').map(|s| s.to_string()).collect();
//                Response::Args(args)
                Response::Args(vec!("yes".to_string()))
            }
            Command::Write(filename, bytes) => {
                let mut file = File::create(filename).unwrap();
                file.write(bytes.as_slice()).unwrap();
                Response::Ack
            }
        }
    }
}