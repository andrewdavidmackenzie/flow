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

use flowrlib::loader::Loader;
use flowrlib::provider::Provider;
use gdk_pixbuf::Pixbuf;
use gio::prelude::*;
use gtk::{
    AboutDialog, AccelFlags, AccelGroup, Application, ApplicationWindow, CheckMenuItem, IconSize, Image, Label,
    Menu, MenuBar, MenuItem, WindowPosition
};
use gtk::prelude::*;
use provider::content::provider::MetaProvider;
use std::env::args;

mod runtime;

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
    if let Ok(image) = Pixbuf::new_from_file(format!("{}/resources/icons/png/128x128.png",
                                env!("CARGO_MANIFEST_DIR"))) {
        p.set_logo(Some(&image));
    }

    //CARGO_PKG_DESCRIPTION
    //CARGO_PKG_REPOSITORY

    // AboutDialog has a secondary credits section
    p.set_authors(&[env!("CARGO_PKG_AUTHORS")]);

    p
}

fn menu_bar(window: &ApplicationWindow) -> MenuBar {
    let menu = Menu::new();
    let accel_group = AccelGroup::new();
    window.add_accel_group(&accel_group);
    let menu_bar = MenuBar::new();
    let file = MenuItem::new_with_label("File");
    let about = MenuItem::new_with_label("About");
    let quit = MenuItem::new_with_label("Quit");
    let file_item = MenuItem::new();
    let file_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let file_image = Image::new_from_file("resources/file.png");
    let file_label = Label::new(Some("File"));
    let folder_item = MenuItem::new();
    let folder_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let folder_image = Image::new_from_icon_name(Some("folder-music-symbolic"), IconSize::Menu);
    let folder_label = Label::new(Some("Folder"));
    let check_item = CheckMenuItem::new_with_label("Click me!");

    check_item.connect_toggled(|w| {
        w.set_label(if w.get_active() {
            "Checked"
        } else {
            "Unchecked"
        });
    });

    file_box.pack_start(&file_image, false, false, 0);
    file_box.pack_start(&file_label, true, true, 0);
    file_item.add(&file_box);
    folder_box.pack_start(&folder_image, false, false, 0);
    folder_box.pack_start(&folder_label, true, true, 0);
    folder_item.add(&folder_box);
    menu.append(&file_item);
    menu.append(&folder_item);
    menu.append(&check_item);
    menu.append(&about);
    menu.append(&quit);
    file.set_submenu(Some(&menu));
    menu_bar.append(&file);

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

fn main_window() -> Label {
    Label::new(Some("MenuBar example"))
}

fn build_ui(application: &gtk::Application) {
    let window = ApplicationWindow::new(application);

    window.set_title(env!("CARGO_PKG_NAME"));
    window.set_position(WindowPosition::Center);
    window.set_size_request(400, 400);

    let v_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
    v_box.pack_start(&menu_bar(&window), false, false, 0);
    v_box.pack_start(&main_window(), true, true, 0);

    window.add(&v_box);

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