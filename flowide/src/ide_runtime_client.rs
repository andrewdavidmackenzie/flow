// use std::fs::File;
// use std::io::prelude::*;
//
// use gtk::TextBufferExt;

// use flowrlib::runtime::{Event, Response};
// use flowrlib::lib.client_server::RuntimeClientConnection;
use std::collections::HashMap;

// use crate::widgets;
use image::{ImageBuffer, Rgb};

#[derive(Debug, Clone)]
pub struct IDERuntimeClient {
    args: Vec<String>,
    image_buffers: HashMap<String, ImageBuffer<Rgb<u8>, Vec<u8>>>,
    display_metrics: bool,
}

impl IDERuntimeClient {
    pub fn new(display_metrics: bool) -> Self {
        IDERuntimeClient {
            args: vec!(),
            image_buffers: HashMap::<String, ImageBuffer<Rgb<u8>, Vec<u8>>>::new(),
            display_metrics,
        }
    }

    /*
        Enter  a loop where we receive events as a client and respond to them
     */
    // pub fn start(connection: RuntimeClientConnection,
    //              #[cfg(feature = "metrics")]
    //              display_metrics: bool,
    // ) {
    //     Self::capture_control_c(&connection);
    //
    //     let mut runtime_client = IDERuntimeClient::new(display_metrics);
    //
    //     loop {
    //         match connection.client_recv() {
    //             Ok(event) => {
    //                 let response = runtime_client.process_event(event);
    //                 if response == Response::ClientExiting {
    //                     return;
    //                 }
    //                 let _ = connection.client_send(response);
    //             }
    //             Err(_) => {
    //                 return;
    //             }
    //         }
    //     }
    // }

//     fn capture_control_c(_connection: &RuntimeClientConnection) {
//         // let connection_clone = connection.clone();
//         // let _ = ctrlc::set_handler(move || {
//         //     let _ = connection_clone.send(Response::EnterDebugger);
//         // });
//     }
//
//     fn process_event(&mut self, event: Event) -> Response {
//         match event {
//             Event::FlowStart => Response::Ack,
//             #[cfg(feature = "debugger")]
//             Event::FlowEnd(_) => Response::Ack,
//             #[cfg(not(feature = "debugger"))]
//             Event::FlowEnd => Response::Ack,
//             Event::StdoutEOF => Response::Ack,
//             Event::Stdout(contents) => {
//                 widgets::do_in_gtk_eventloop(|refs| {
//                     refs.stdout().insert_at_cursor(&format!("{}\n", contents));
//                 });
//                 Response::Ack
//             }
//             Event::Stderr(contents) => {
//                 widgets::do_in_gtk_eventloop(|refs| {
//                     refs.stderr().insert_at_cursor(&format!("{}\n", contents));
//                 });
//                 Response::Ack
//             }
//             Event::GetStdin => {
// //                Response::Stdin("bla bla".to_string()) // TODO
//                 Response::Error("Could not read Stdin".into())
//             }
//             Event::GetLine => {
//                 Response::Stdin("bla bla".to_string())  // TODO
//             }
//             Event::GetArgs => {
//                 Response::Args(self.args.clone())
//             }
//             Event::Write(filename, bytes) => {
//                 let mut file = File::create(filename).unwrap();
//                 file.write_all(bytes.as_slice()).unwrap();
//                 Response::Ack
//             }
//             Event::PixelWrite((_x, _y), (_r, _g, _b), (_width, _height), _name) => {
//                 // let image = self.image_buffers.entry(name)
//                 //     .or_insert(RgbImage::new(width, height));
//                 // image.put_pixel(x, y, Rgb([r, g, b]));
//                 Response::Ack
//             }
//             Event::StderrEOF => Response::Ack,
//         }
//     }
//
//     // This function is called by the runtime_function to send a command to the runtime_client
//     pub fn send_event(&mut self, event: Event) -> Response {
//         self.process_event(event)
//     }

    pub fn set_args(&mut self, args: Vec<String>) {
        self.args = args;
    }
}