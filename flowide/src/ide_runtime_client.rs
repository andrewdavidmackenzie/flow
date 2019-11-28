use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender};

use glib;

use runtime::runtime_client::{Command, Response, RuntimeClient};

pub struct IDERuntimeClient {
    command_sender: Arc<Mutex<glib::Sender<Command>>>,
    response_sender: Arc<Mutex<Sender<Response>>>,
    response_receiver: Arc<Mutex<Receiver<Response>>>
}

impl RuntimeClient for IDERuntimeClient {
    fn init(&self) {}

    // This function is called by the runtime_function to send a commanmd to the runtime_client
    fn send_command(&self, command: Command) -> Response {
        self.command_sender.lock().unwrap().send(command).unwrap(); // TODO Result return type

        // wait for response back on the channel from the UI thread and return it to the runtime_function
        self.response_receiver.lock().unwrap().recv().unwrap() // TODO
    }
}

/*
    This processes a command, interacts with the UI Widgets needed and then returns a response
*/
impl IDERuntimeClient {
    pub fn new(command_sender: glib::Sender<Command>) -> Self {
        let (response_sender, response_receiver) = channel();

        IDERuntimeClient {
            command_sender: Arc::new(Mutex::new(command_sender)),
            response_sender: Arc::new(Mutex::new(response_sender)),
            response_receiver: Arc::new(Mutex::new(response_receiver)),
        }
    }

    /*
        This function should run on the UI thread as it needs to interact with UI Widgets
    */
    pub fn process_command(&self, command: Command) {
        let response = match command {
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
                    if size > 0 {
                        Response::Stdin(buffer.trim().to_string());
                    }
                }
                Response::Error("Could not read Stdin".into())
            }
            Command::Readline => {
                let mut input = String::new();
                match io::stdin().read_line(&mut input) {
                    Ok(n) if n > 0 => Response::Readline(input.trim().to_string()),
                    _ => Response::Error("Could not read Readline".into())
                }
            }
            Command::Args => {
                Response::Args(vec!()) // TODO
            }
            Command::Write(filename, bytes) => {
                let mut file = File::create(filename).unwrap();
                file.write(bytes.as_slice()).unwrap();
                Response::Ack
            }
        };

        // send response back on the channel to original thread
        self.response_sender.lock().unwrap().send(response).unwrap(); // TODO
    }
}