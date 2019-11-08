#![deny(missing_docs)]
//! The `ide-native` is a prototype of a native IDE for `flow` programs.

extern crate druid;
extern crate flow_impl;
extern crate flowclib;
extern crate flowrlib;
#[macro_use]
extern crate serde_json;

use druid::{AppLauncher, Data, LocalizedString, MenuDesc, theme, Widget, WindowDesc};
use druid::piet::Color;
use druid::widget::{Column, DynLabel, Padding, TextBox};

mod runtime;

fn main() {
    let flowclib_version = flowclib::info::version();
    let flowrlib_version = flowrlib::info::version();

    let window = WindowDesc::new(build_widget).menu(make_main_menu());

    AppLauncher::with_window(window)
        .configure_env(|env| {
            env.set(theme::SELECTION_COLOR, Color::rgb8(0xA6, 0xCC, 0xFF));
            env.set(theme::WINDOW_BACKGROUND_COLOR, Color::WHITE);
            env.set(theme::LABEL_COLOR, Color::BLACK);
            env.set(theme::CURSOR_COLOR, Color::BLACK);
            env.set(theme::BACKGROUND_LIGHT, Color::rgb8(230, 230, 230));
        })
        .use_simple_logger()
        .launch(flowclib_version.to_string())
        .expect("launch failed");
}

fn build_widget() -> impl Widget<String> {
    let mut col = Column::new();

    let textbox = TextBox::new();
    let textbox_2 = TextBox::new();
    let label = DynLabel::new(|data: &String, _env| format!("value: {}", data));

    col.add_child(Padding::new(5.0, textbox), 1.0);
    col.add_child(Padding::new(5.0, textbox_2), 1.0);
    col.add_child(Padding::new(5.0, label), 1.0);
    col
}

fn make_main_menu<T: Data>() -> MenuDesc<T> {
    let edit_menu = MenuDesc::new(LocalizedString::new("common-menu-edit-menu"))
        .append(druid::menu::sys::common::cut())
        .append(druid::menu::sys::common::copy())
        .append(druid::menu::sys::common::paste());

    MenuDesc::platform_default()
        .unwrap_or(MenuDesc::empty())
        .append(edit_menu)
}
