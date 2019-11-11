#![deny(missing_docs)]
//! The `flowide` is a prototype of a native IDE for `flow` programs.

extern crate flow_impl;
extern crate flowclib;
extern crate flowrlib;
extern crate gio;
extern crate gtk;
extern crate provider;
#[macro_use]
extern crate serde_json;

use flowrlib::loader::Loader;
use flowrlib::provider::Provider;
use gio::prelude::*;
use gtk::{Application, ApplicationWindow, Label};
use gtk::prelude::*;
use provider::content::provider::MetaProvider;
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

fn load_libs(loader: &mut Loader, provider: &dyn Provider) -> Result<(), String> {
    // Load this runtime's library of native (statically linked) implementations
    loader.add_lib(provider, runtime::manifest::get_manifest(), "runtime")
        .map_err(|e| e.to_string())?;

    // If the "native" feature is enabled then load the native flowstdlib if command line arg to do so
    loader.add_lib(provider, flowstdlib::get_manifest(), "flowstdlib")
        .map_err(|e| e.to_string())
}

fn main() {
    let application = Application::new(
        Some("net.mackenzie-serres.flow.ide"),
        Default::default(),
    ).expect("failed to initialize GTK application");

    application.connect_activate(|app| {
        build_ui(app);
    });

    let mut loader = Loader::new();
    let provider = MetaProvider {};

    let _result = load_libs(&mut loader, &provider);

    application.run(&args().collect::<Vec<_>>());
}