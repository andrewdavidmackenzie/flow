#![deny(missing_docs)]
//! The `ide-native` is a prototype of a native IDE for `flow` programs.

extern crate flow_impl;
extern crate flowclib;
extern crate flowrlib;
extern crate gio;
extern crate gtk;
#[macro_use]
extern crate serde_json;

use gio::prelude::*;
use gtk::{Application, ApplicationWindow, Button};
use gtk::prelude::*;

mod runtime;

fn main() {
    let flowclib_version = flowclib::info::version();
    let flowrlib_version = flowrlib::info::version();

    let application = Application::new(
        Some("com.github.gtk-rs.examples.basic"),
        Default::default(),
    ).expect("failed to initialize GTK application");

    application.connect_activate(|app| {
        let window = ApplicationWindow::new(app);
        window.set_title("First GTK+ Program");
        window.set_default_size(350, 70);

        let button = Button::new_with_label("Click me!");
        button.connect_clicked(|_| {
            println!("Clicked!");
        });
        window.add(&button);

        window.show_all();
    });

    application.run(&[]);
}