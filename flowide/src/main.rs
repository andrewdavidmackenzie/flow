#![deny(missing_docs)]
//! The `flowide` is a prototype of a native IDE for `flow` programs.

extern crate flow_impl;
extern crate flowclib;
extern crate flowrlib;
extern crate gdk_pixbuf;
extern crate gio;
extern crate gtk;
extern crate provider;
#[macro_use]
extern crate serde_json;

use flowclib::deserializers::deserializer_helper;
use flowrlib::loader::Loader;
use flowrlib::provider::Provider;
use gdk_pixbuf::Pixbuf;
use gio::prelude::*;
use gtk::{
    AboutDialog, AccelFlags, AccelGroup, Application, ApplicationWindow, Box, FileChooserAction, FileChooserDialog,
    FileFilter, Menu, MenuBar, MenuItem, ResponseType, ScrolledWindow, TextBuffer, TextView, Widget, WidgetExt, WindowPosition
};
use gtk::prelude::*;
use provider::content::provider::MetaProvider;
use std::env::args;

mod runtime;

/// `RuntimeCOntext` Holds items from the UI that are needed during runnings of a slow.
pub struct RuntimeContext {
    args: TextBuffer,
    stdout: TextBuffer,
    stderr: TextBuffer
}

/// upgrade weak reference or return
#[macro_export]
macro_rules! upgrade_weak {
    ($x:ident, $r:expr) => {{
        match $x.upgrade() {
            Some(o) => o,
            None => return $r,
        }
    }};
    ($x:ident) => {
        upgrade_weak!($x, ())
    };
}

fn resource(path: &str) -> String {
    format!("{}/resources/{}", env!("CARGO_MANIFEST_DIR"), path)
}

fn about_dialog() -> AboutDialog {
    let p = AboutDialog::new();
    p.set_program_name(env!("CARGO_PKG_NAME"));
    p.set_website_label(Some("Flow website"));
    p.set_website(Some(env!("CARGO_PKG_HOMEPAGE")));
    p.set_title(&format!("About {}", env!("CARGO_PKG_NAME")));
    p.set_version(Some(env!("CARGO_PKG_VERSION")));
    p.set_comments(Some(&format!("flowclib version: {}\nflowrlib version: {}",
                                 flowclib::info::version(), flowrlib::info::version())));
    println!("pwd {:?}", std::env::current_dir());
    if let Ok(image) = Pixbuf::new_from_file(resource("icons/png/128x128.png")) {
        p.set_logo(Some(&image));
    }

    //CARGO_PKG_DESCRIPTION
    //CARGO_PKG_REPOSITORY

    // AboutDialog has a secondary credits section
    p.set_authors(&[env!("CARGO_PKG_AUTHORS")]);

    p
}

fn menu_bar(window: &ApplicationWindow, extensions: &'static [&'static str]) -> MenuBar {
    let menu = Menu::new();
    let accel_group = AccelGroup::new();
    window.add_accel_group(&accel_group);
    let menu_bar = MenuBar::new();

    let file = MenuItem::new_with_label("File");
    let open = MenuItem::new_with_label("Open");
    let about = MenuItem::new_with_label("About");
    let quit = MenuItem::new_with_label("Quit");

    menu.append(&open);
    menu.append(&about);
    menu.append(&quit);
    file.set_submenu(Some(&menu));
    menu_bar.append(&file);

    let window_weak = window.downgrade();
    open.connect_activate(move |_| {
        let window = upgrade_weak!(window_weak);

        let dialog = FileChooserDialog::new(Some("Choose a file"), Some(&window),
                                            FileChooserAction::Open);
        dialog.add_buttons(&[
            ("Open", ResponseType::Ok),
            ("Cancel", ResponseType::Cancel)
        ]);

        dialog.set_select_multiple(false);
        let filter = FileFilter::new();
        for extension in extensions {
            filter.add_pattern(&format!("*.{}", extension));
        }
        dialog.set_filter(&filter);
        dialog.run();
        let uris = dialog.get_uris();
        dialog.destroy();

        if let Some(uri) = uris.get(0) {
            println!("Uri: {:?}", uri.to_string());
        }
    });

    let other_menu = Menu::new();
    let sub_other_menu = Menu::new();
    let other = MenuItem::new_with_label("Another");
    let sub_other = MenuItem::new_with_label("Sub another");
    let sub_other2 = MenuItem::new_with_label("Sub another 2");
    let sub_sub_other2 = MenuItem::new_with_label("Sub sub another 2");
    let sub_sub_other2_2 = MenuItem::new_with_label("Sub sub another2 2");

    sub_other_menu.append(&sub_sub_other2);
    sub_other_menu.append(&sub_sub_other2_2);
    sub_other2.set_submenu(Some(&sub_other_menu));
    other_menu.append(&sub_other);
    other_menu.append(&sub_other2);
    other.set_submenu(Some(&other_menu));
    menu_bar.append(&other);

    let window_weak = window.downgrade();
    quit.connect_activate(move |_| {
        let window = upgrade_weak!(window_weak);
        window.destroy();
    });

    // `Primary` is `Ctrl` on Windows and Linux, and `command` on macOS
    // It isn't available directly through gdk::ModifierType, since it has
    // different values on different platforms.
    let (key, modifier) = gtk::accelerator_parse("<Primary>Q");
    quit.add_accelerator("activate", &accel_group, key, modifier, AccelFlags::VISIBLE);

    let window_weak = window.downgrade();
    about.connect_activate(move |_| {
        let ad = about_dialog();
        let window = upgrade_weak!(window_weak);
        ad.set_transient_for(Some(&window));
        ad.run();
        ad.destroy();
    });

    menu_bar
}

fn args_view() -> (TextView, TextBuffer) {
    let args_view = gtk::TextView::new();
    args_view.set_size_request(-1,1); // Want to fill width and be one line high :-(
    let args_buffer = args_view.get_buffer().unwrap();
    (args_view, args_buffer)
}

fn stdio() -> (ScrolledWindow, TextBuffer) {
    let scroll = gtk::ScrolledWindow::new(gtk::NONE_ADJUSTMENT, gtk::NONE_ADJUSTMENT);
    let view = gtk::TextView::new();
    view.set_editable(false);
    let buffer = view.get_buffer().unwrap();
    scroll.add(&view);
    (scroll, buffer)
}

fn main_window() -> (Box, RuntimeContext) {
    let main = gtk::Box::new(gtk::Orientation::Vertical, 10);
    main.set_border_width(6);
    main.set_vexpand(true);
    main.set_hexpand(true);

    let (args_view, args_buffer) = args_view();
    let (stdout_view, stdout_buffer) = stdio();
    let (stderr_view, stderr_buffer) = stdio();

    main.pack_start(&args_view, true, true, 0);
    main.pack_start(&stdout_view, true, true, 0);
    main.pack_start(&stderr_view, true, true, 0);

    let context = RuntimeContext {
        args: args_buffer,
        stdout: stdout_buffer,
        stderr: stderr_buffer
    };
    (main, context)
}

fn build_ui<W: IsA<Widget>>(application: &gtk::Application,
            extensions: &'static [&'static str],
            main_window: &W) {
    let window = ApplicationWindow::new(application);

    window.set_title(env!("CARGO_PKG_NAME"));
    window.set_position(WindowPosition::Center);
    window.set_size_request(400, 400);

    let v_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
    v_box.pack_start(&menu_bar(&window, extensions), false, false, 0);
    v_box.pack_start(main_window, true, true, 0);

    window.add(&v_box);

    window.show_all();
}

fn load_libs(loader: &mut Loader,
             provider: &dyn Provider,
             runtime_context: &RuntimeContext) -> Result<(), String> {
    // Load this runtime's library of native (statically linked) implementations
    loader.add_lib(provider, runtime::manifest::create_runtime(runtime_context), "runtime")
        .map_err(|e| e.to_string())?;

    // If the "native" feature is enabled then load the native flowstdlib if command line arg to do so
    loader.add_lib(provider, flowstdlib::get_manifest(), "flowstdlib")
        .map_err(|e| e.to_string())
}

fn run_flow(runtime_context: &RuntimeContext) {
    let mut loader = Loader::new();
    let provider = MetaProvider {};
    let _result = load_libs(&mut loader, &provider, &runtime_context);
}

fn main() {
    let application = Application::new(
        Some("net.mackenzie-serres.flow.ide"),
        Default::default()).expect("failed to initialize GTK application");

    let (main_window, runtime_context) = main_window();

    application.connect_activate(move |app| {
        let accepted_extensions = deserializer_helper::get_accepted_extensions();
        build_ui(&app, accepted_extensions, &main_window);
    });

    runtime_context.stdout.insert_at_cursor("hello\n");
    runtime_context.stdout.insert_at_cursor("world\n");

    application.run(&args().collect::<Vec<_>>());
}