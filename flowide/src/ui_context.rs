use glib;

use flowrlib::manifest::Manifest;
use runtime::runtime_client::Command;

use crate::ide_runtime_client::IDERuntimeClient;

pub struct UIContext {
    command_receiver: glib::Receiver<Command>,
    runtime_client: IDERuntimeClient,
    pub manifest: Option<Manifest>,
}

impl UIContext {
    pub fn new() -> Self {
        let (command_sender, command_receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        UIContext {
            command_receiver,
            runtime_client: IDERuntimeClient::new(command_sender),
            manifest: None,
        }
    }
}