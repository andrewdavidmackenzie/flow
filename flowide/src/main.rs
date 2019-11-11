#![deny(missing_docs)]
//! The `flowide` is a prototype of a native IDE for `flow` programs.

extern crate flow_impl;
extern crate flowclib;
extern crate flowrlib;
extern crate gio;
extern crate gtk;
#[macro_use]
extern crate serde_json;

use gio::prelude::*;
use gtk::{Application, ApplicationWindow, Label};
use gtk::prelude::*;
use std::env::args;

mod runtime;

fn build_ui(app: &gtk::Application) {
    let window = ApplicationWindow::new(app);
    window.set_title(env!("CARGO_PKG_NAME"));
    window.set_default_size(350, 70);

    let mut label = Label::new(Some(&format!("flowclib version: {}", flowclib::info::version())));
    window.add(&label);

    label = Label::new(Some(&format!("flowrlib version: {}", flowrlib::info::version())));
    window.add(&label);

    window.show_all();
}

fn main() {
    let application = Application::new(
        Some("net.mackenzie-serres.flow.ide"),
        Default::default(),
    ).expect("failed to initialize GTK application");

    application.connect_activate(|app| {
        build_ui(app);
    });

    application.run(&args().collect::<Vec<_>>());
}