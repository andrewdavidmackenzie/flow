#![deny(missing_docs)]
//! The `flowide` is a prototype of a native IDE for `flow` programs.

use flowclib::deserializers::deserializer_helper;
use flowrlib::loader::Loader;
use flowrlib::provider::Provider;
use gdk_pixbuf::Pixbuf;
use gio::prelude::*;
use gtk::{
    AboutDialog, AccelFlags, AccelGroup, Application, ApplicationWindow, Box, FileChooserAction, FileChooserDialog,
    FileFilter, Menu, MenuBar, MenuItem, ResponseType, ScrolledWindow, TextBuffer, TextView, WidgetExt, WindowPosition,
};
use gtk::prelude::*;
use provider::content::provider::MetaProvider;
use runtime::runtime_client::{Command, Response, RuntimeClient};
use runtime_context::RuntimeContext;
use std::env;
use std::env::args;
use std::fs::File;
use std::io;
use std::io::prelude::*;

mod runtime_context;

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

fn run_flow(_runtime_context: &RuntimeContext) {
    println!("Run");
}

fn file_run_action(_run: &MenuItem, runtime_context: &RuntimeContext) {
//        run.connect_activate( |_| {
    run_flow(runtime_context);
//    });
}

fn file_open_action(window: &ApplicationWindow, open: &MenuItem) {
    let accepted_extensions = deserializer_helper::get_accepted_extensions();

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
        for extension in accepted_extensions {
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
}

fn menu_bar(window: &ApplicationWindow, runtime_context: &RuntimeContext) -> MenuBar {
    let menu = Menu::new();
    let accel_group = AccelGroup::new();
    window.add_accel_group(&accel_group);
    let menu_bar = MenuBar::new();

    let file = MenuItem::new_with_label("File");
    let open = MenuItem::new_with_label("Open");
    let about = MenuItem::new_with_label("About");
    let run = MenuItem::new_with_label("Run");
    let quit = MenuItem::new_with_label("Quit");

    menu.append(&open);
    menu.append(&about);
    menu.append(&run);
    menu.append(&quit);
    file.set_submenu(Some(&menu));
    menu_bar.append(&file);

    file_open_action(window, &open);
    file_run_action(&run, runtime_context);

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

fn args_view(buffer: &TextBuffer) -> TextView {
    let args_view = gtk::TextView::new();
    args_view.set_buffer(Some(buffer));
    args_view.set_size_request(-1, 1); // Want to fill width and be one line high :-(
    args_view
}

fn stdio(buffer: &TextBuffer) -> ScrolledWindow {
    let scroll = gtk::ScrolledWindow::new(gtk::NONE_ADJUSTMENT, gtk::NONE_ADJUSTMENT);
    let view = gtk::TextView::new();
    view.set_buffer(Some(buffer));
    view.set_editable(false);
    scroll.add(&view);
    scroll
}

fn main_window(runtime_context: &RuntimeContext) -> Box {
    let main = gtk::Box::new(gtk::Orientation::Vertical, 10);
    main.set_border_width(6);
    main.set_vexpand(true);
    main.set_hexpand(true);

    let args_view = args_view(runtime_context.args);
    let stdout_view = stdio(runtime_context.stdout);
    let stderr_view = stdio(runtime_context.stderr);

    main.pack_start(&args_view, true, true, 0);
    main.pack_start(&stdout_view, true, true, 0);
    main.pack_start(&stderr_view, true, true, 0);

    main
}

fn build_ui(application: &gtk::Application, runtime_context: &RuntimeContext) {
    let main_window = main_window(&runtime_context);

    let app_window = ApplicationWindow::new(application);

    app_window.set_title(env!("CARGO_PKG_NAME"));
    app_window.set_position(WindowPosition::Center);
    app_window.set_size_request(400, 400);

    let v_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
    v_box.pack_start(&menu_bar(&app_window, &runtime_context), false, false, 0);
    v_box.pack_start(&main_window, true, true, 0);

    app_window.add(&v_box);

    app_window.show_all();
}

fn load_libs<'a>(loader: &'a mut Loader, provider: &dyn Provider, client: &'a dyn RuntimeClient) -> Result<(), String> {
    let client_mutex = Arc::new(Mutex::new(client));
    let runtime_manifest = runtime::manifest::create_runtime(client);

    // Load this runtime's library of native (statically linked) implementations
    loader.add_lib(provider, runtime_manifest, "runtime").map_err(|e| e.to_string())?;

    // If the "native" feature is enabled then load the native flowstdlib if command line arg to do so
    loader.add_lib(provider, flowstdlib::get_manifest(), "flowstdlib").map_err(|e| e.to_string())?;

    Ok(())
}

struct IDE {}

impl RuntimeClient for IDE {
    fn init(&self) {}

    // This function is called by the runtime_function to send a commanmd to the runtime_client
    // so here in the runtime_client, it's more like "process_command"
    fn send_command(&self, command: Command) -> Response {
        match command {
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
                        return Response::Stdin(buffer.trim().to_string());
                    }
                }
                return Response::Error("Could not read Stdin".into());
            }
            Command::Readline => {
                let mut input = String::new();
                match io::stdin().read_line(&mut input) {
                    Ok(n) if n > 0 => return Response::Readline(input.trim().to_string()),
                    _ => return Response::Error("Could not read Readline".into())
                }
            }
            Command::Args => {
                return Response::Args(vec!()); // TODO
            }
            Command::Write(filename, bytes) => {
                let mut file = File::create(filename).unwrap();
                file.write(bytes.as_slice()).unwrap();
                return Response::Ack;
            }
        }
    }
}

fn main() -> Result<(), String> {
    let mut loader = Loader::new();
    let provider = MetaProvider {};
    let ide = IDE {};

    load_libs(&mut loader, &provider, &ide).map_err(|e| e.to_string())?;

    let application = Application::new(Some("net.mackenzie-serres.flow.ide"), Default::default())
        .expect("failed to initialize GTK application");

    let runtime_context = RuntimeContext::new(&TextBuffer::new(gtk::NONE_TEXT_TAG_TABLE),
                                                  &TextBuffer::new(gtk::NONE_TEXT_TAG_TABLE),
                                                  &TextBuffer::new(gtk::NONE_TEXT_TAG_TABLE));

    application.connect_activate(move |app| {
        build_ui(&app, &runtime_context);
    });

    application.run(&args().collect::<Vec<_>>());

    Ok(())
}