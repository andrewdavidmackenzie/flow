//#![deny(missing_docs)]
//! The `flowide` is a prototype of a native IDE for `flow` programs.

use std::env;
use std::sync::{Arc, Mutex};

use gdk_pixbuf::Pixbuf;
use gio::prelude::*;
use gtk::{
    AboutDialog, AccelFlags, AccelGroup, Application, ApplicationWindow, FileChooserAction, FileChooserDialog,
    FileFilter, Menu, MenuBar, MenuItem, ResponseType, ScrolledWindow, TextBuffer, TextBufferExt, TextView, WidgetExt, WindowPosition,
};
use gtk::prelude::*;
use gtk_fnonce_on_eventloop::gtk_refs;
use toml;

use flowclib::deserializers::deserializer_helper;
use flowrlib::coordinator::{Coordinator, Submission};
use ide_runtime_client::IDERuntimeClient;
use lazy_static::lazy_static;
use ui_context::UIContext;

mod ide_runtime_client;
mod ui_context;
mod actions;

lazy_static! {
    static ref UICONTEXT: Arc<Mutex<UIContext>> = Arc::new(Mutex::new(UIContext::new()));
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

gtk_refs!(
    pub mod widgets;                // The macro emits a new module with this name
    struct WidgetRefs;              // The macro emits a struct with this name containing:
    app_window: gtk::ApplicationWindow,
    main_window: gtk::Box,
    flow_buffer: gtk::TextBuffer,
    manifest_buffer: gtk::TextBuffer,
    stdout: gtk::TextBuffer,
    stderr: gtk::TextBuffer
);

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

fn open_flow(uri: String) {
    std::thread::spawn(move || {
        let flow = actions::load_flow(&uri).unwrap(); // TODO

//        let flow_content = toml::to_string_pretty(&flow); // TODO
        let flow_content =  toml::Value::try_from(&flow).unwrap().to_string();
        println!("flow_content in toml : {}", flow_content);
        match UICONTEXT.lock() {
            Ok(mut context) => {
                context.flow = Some(flow);
                // TODO enable run action
                // TODO need to add a method to compile from string contents not a URL?
                // what about urls pointing to sub parts? need to force save first?
                // NOT allow editing in IDE, just viewing the url?
            }
            Err(_) => { /* TODO */ }
        }

        widgets::do_in_gtk_eventloop(|refs|
            refs.flow_buffer().set_text(&flow_content)
        );
    });
}

fn open_manifest(uri: String) {
    std::thread::spawn(move || {
        let runtime_client = Arc::new(Mutex::new(IDERuntimeClient));
        let (loader, manifest) = actions::load_from_uri(&uri, runtime_client).unwrap(); // TODO

        let manifest_content = serde_json::to_string_pretty(&manifest).unwrap(); // TODO

        match UICONTEXT.lock() {
            Ok(mut context) => {
                context.loader = Some(loader);
                context.manifest = Some(manifest);
                // TODO enable run action
            }
            Err(_) => {}
        }

        widgets::do_in_gtk_eventloop(|refs|
            refs.manifest_buffer().set_text(&manifest_content)
        );
    });
}

fn open_action<F: 'static>(window: &ApplicationWindow, open: &MenuItem, func: F)
    where F: Fn(String) {
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
            func(uri.to_string());
        }
    });
}

fn run_manifest() -> Result<String, String> {
    match UICONTEXT.lock() {
        Ok(ref mut context) => {
            match &context.manifest {
                Some(manifest) => {
                    let manifest_clone = manifest.clone();
                    std::thread::spawn(move || {
                        let submission = Submission::new(manifest_clone, 1, false, None);
                        let mut coordinator = Coordinator::new(1);
                        coordinator.submit(submission);
                    });
                    Ok("Submitting flow for execution".to_string()) // TODO useless for now as it's blocked running it
                }
                _ => Err("No manifest loaded to run".into())
            }
        }
        _ => Err("Could not access ui context".into())
    }
}

// run the loaded manifest from run menu item
fn run_action(run: &MenuItem) {
    run.connect_activate(move |_| {
        let _ = run_manifest().unwrap(); // TODO
    });
}

// compile the loaded flow
fn compile_action(compile: &MenuItem) {
    compile.connect_activate(move |_| {
        // compile_flow(flow: &Flow, debug_symbols: bool, manifest_dir: &str) -> Result<Manifest, String>
        // TODO capture output and store in manifest and update manifest widget
    });
}

fn menu_bar(widget_refs: &widgets::WidgetRefs) -> MenuBar {
    let accel_group = AccelGroup::new();
    widget_refs.app_window.add_accel_group(&accel_group);
    let menu_bar = MenuBar::new();

    // File Menu
    let file_menu = Menu::new();
    let file = MenuItem::new_with_label("File");
    let about = MenuItem::new_with_label("About");
    let quit = MenuItem::new_with_label("Quit");
    file_menu.append(&about);
    file_menu.append(&quit);
    file.set_submenu(Some(&file_menu));
    // `Primary` is `Ctrl` on Windows and Linux, and `command` on macOS
    let (key, modifier) = gtk::accelerator_parse("<Primary>Q");
    quit.add_accelerator("activate", &accel_group, key, modifier, AccelFlags::VISIBLE);
    menu_bar.append(&file);

    // Flow Menu
    let flow_menu = Menu::new();
    let flow = MenuItem::new_with_label("Flow");
    let open_flow_menu_item = MenuItem::new_with_label("Open");
    let compile_flow_menu_item = MenuItem::new_with_label("Compile");
    flow_menu.append(&open_flow_menu_item);
    flow_menu.append(&compile_flow_menu_item);
    flow.set_submenu(Some(&flow_menu));
    open_action(&widget_refs.app_window, &open_flow_menu_item, open_flow);
    compile_action(&compile_flow_menu_item);
    menu_bar.append(&flow);

    // Manifest Menu
    let manifest_menu = Menu::new();
    let manifest = MenuItem::new_with_label("Manifest");
    let open_manifest_menu = MenuItem::new_with_label("Open");
    let run_manifest_menu = MenuItem::new_with_label("Run");
    manifest_menu.append(&open_manifest_menu);
    manifest_menu.append(&run_manifest_menu);
    manifest.set_submenu(Some(&manifest_menu));
    open_action(&widget_refs.app_window, &open_manifest_menu, open_manifest);
    run_action(&run_manifest_menu);
    let (key, modifier) = gtk::accelerator_parse("<Primary>R");
    run_manifest_menu.add_accelerator("activate", &accel_group, key, modifier, AccelFlags::VISIBLE);
    menu_bar.append(&manifest);

    let window_weak = widget_refs.app_window.downgrade();
    quit.connect_activate(move |_| {
        let window = upgrade_weak!(window_weak);
        window.destroy();
    });

    let window_weak = widget_refs.app_window.downgrade();
    about.connect_activate(move |_| {
        let ad = about_dialog();
        let window = upgrade_weak!(window_weak);
        ad.set_transient_for(Some(&window));
        ad.run();
        ad.destroy();
    });

    menu_bar
}

fn args_view() -> TextView {
    let args_view = gtk::TextView::new();
    args_view.set_size_request(-1, 1); // Want to fill width and be one line high :-(
    args_view
}

fn stdio() -> (ScrolledWindow, TextBuffer) {
    let scroll = gtk::ScrolledWindow::new(gtk::NONE_ADJUSTMENT, gtk::NONE_ADJUSTMENT);
    let view = gtk::TextView::new();
    view.set_editable(false);
    scroll.add(&view);
    (scroll, view.get_buffer().unwrap())
}

fn flow_viewer() -> (ScrolledWindow, TextBuffer) {
    let scroll = gtk::ScrolledWindow::new(gtk::NONE_ADJUSTMENT, gtk::NONE_ADJUSTMENT);
    let view = gtk::TextView::new();
    view.set_editable(false);
    scroll.add(&view);
    (scroll, view.get_buffer().unwrap())
}

fn manifest_viewer() -> (ScrolledWindow, TextBuffer) {
    let scroll = gtk::ScrolledWindow::new(gtk::NONE_ADJUSTMENT, gtk::NONE_ADJUSTMENT);
    let view = gtk::TextView::new();
    view.set_editable(false);
    scroll.add(&view);
    (scroll, view.get_buffer().unwrap())
}

fn main_window(app_window: &ApplicationWindow) -> widgets::WidgetRefs {
    let main_window = gtk::Box::new(gtk::Orientation::Vertical, 10);
    main_window.set_border_width(6);
    main_window.set_vexpand(true);
    main_window.set_hexpand(true);

    let (flow_view, flow_buffer) = flow_viewer();
    let (manifest_view, manifest_buffer) = manifest_viewer();
    let args_view = args_view();
    let (stdout_view, stdout_buffer) = stdio();
    let (stderr_view, stderr_buffer) = stdio();

    main_window.pack_start(&flow_view, true, true, 0);
    main_window.pack_start(&manifest_view, true, true, 0);
    main_window.pack_start(&args_view, true, true, 0);
    main_window.pack_start(&stdout_view, true, true, 0);
    main_window.pack_start(&stderr_view, true, true, 0);

    widgets::WidgetRefs {
        app_window: app_window.clone(),
        main_window,
        flow_buffer,
        manifest_buffer,
        stdout: stdout_buffer,
        stderr: stderr_buffer,
    }
}

fn build_ui(application: &Application) {
    let app_window = ApplicationWindow::new(application);
    app_window.set_title(env!("CARGO_PKG_NAME"));
    app_window.set_position(WindowPosition::Center);
    app_window.set_size_request(400, 400);

    app_window.connect_delete_event(move |_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    let widget_refs = main_window(&app_window);

    let v_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
    v_box.pack_start(&menu_bar(&widget_refs), false, false, 0);
    v_box.pack_start(&widget_refs.main_window, true, true, 0);

    app_window.add(&v_box);

    app_window.show_all();

    widgets::init_storage(widget_refs);
}

fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function to get better error messages if we ever panic.
    #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();
}

fn main() -> Result<(), String> {
    if gtk::init().is_err() {
        return Err("Failed to initialize GTK.".to_string());
    }

// TODO  read log level from UI or args and log to a logging UI widget?
//    let log_level_arg = get_log_level(&document);
//    init_logging(log_level_arg);

    set_panic_hook();

    let application = Application::new(Some("net.mackenzie-serres.flow.ide"), Default::default())
        .expect("failed to initialize GTK application");

    application.connect_activate(move |app|
        build_ui(app)
    );

    application.run(&std::env::args().collect::<Vec<_>>());

    Ok(())
}

// TODO Read flow lib path from env and add it as a setting with a dialog to edit it.

// load a flow definition
//    let flow_lib_path = get_flow_lib_path(&document).map_err(|e| JsValue::from_str(&e.to_string()))?;
//    let flow = load_flow(&provider, "file:://Users/andrew/workspace/flow/flowide/src/hello_world.toml")
//        .map_err(|e| JsValue::from_str(&e.to_string()))?;

// compile to manifest
//    manifest = compile(&flow, true, "/Users/andrew/workflow/flow")
//        .map_err(|e| JsValue::from_str(&e.to_string()))?;

// or load a manifest from file