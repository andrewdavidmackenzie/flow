use std::collections::HashMap;
use std::fs;

use iced::widget::image::{FilterMethod, Handle, Viewer};
use iced::widget::operation::{self, RelativeOffset};
use iced::widget::scrollable::Scrollable;
use iced::widget::TextInput;
use iced::widget::{text, toggler, Button, Column, Id, Row, Text};
use iced::{ContentFit, Element, Length, Task};
use iced_aw::{TabLabel, Tabs};
use log::error;
use once_cell::sync::Lazy;

use crate::{ImageReference, Message};

#[allow(clippy::struct_field_names)]
pub(crate) struct TabSet {
    pub active_tab: usize,
    pub stdout_tab: StdOutTab,
    pub stderr_tab: StdOutTab,
    pub stdin_tab: StdInTab,
    pub images_tab: ImageTab,
    pub fileio_tab: StdOutTab,
    pub flow_name: String,
}

impl TabSet {
    pub(crate) fn new() -> Self {
        TabSet {
            active_tab: 0,
            stdout_tab: StdOutTab {
                name: "Stdout".to_owned(),
                id: Lazy::new(Id::unique).clone(),
                content: vec![],
                auto_scroll: true,
                unread_count: 0,
            },
            stderr_tab: StdOutTab {
                name: "Stderr".to_owned(),
                id: Lazy::new(Id::unique).clone(),
                content: vec![],
                auto_scroll: true,
                unread_count: 0,
            },
            stdin_tab: StdInTab::new("Stdin"),
            images_tab: ImageTab::new("Images"),
            fileio_tab: StdOutTab {
                name: "FileIO".to_owned(),
                id: Lazy::new(Id::unique).clone(),
                content: vec![],
                auto_scroll: true,
                unread_count: 0,
            },
            flow_name: String::new(),
        }
    }

    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TabSelected(tab_index) => {
                self.active_tab = tab_index;
                match tab_index {
                    0 if self.stdout_tab.auto_scroll => self.stdout_tab.unread_count = 0,
                    1 if self.stderr_tab.auto_scroll => self.stderr_tab.unread_count = 0,
                    3 => self.images_tab.new_activity = false,
                    4 if self.fileio_tab.auto_scroll => self.fileio_tab.unread_count = 0,
                    _ => {}
                }
            }
            Message::ClearTab(ref name) => {
                if name == &self.stdout_tab.name {
                    self.stdout_tab.clear();
                } else if name == &self.stderr_tab.name {
                    self.stderr_tab.clear();
                } else if name == &self.fileio_tab.name {
                    self.fileio_tab.clear();
                }
            }
            Message::StdioAutoScrollTogglerChanged(id, value) => {
                if id == self.stdout_tab.id {
                    self.stdout_tab.auto_scroll = value;
                } else {
                    self.stderr_tab.auto_scroll = value;
                }

                if value {
                    return operation::snap_to(id, RelativeOffset::END);
                }
            }
            Message::SaveTabContent(ref name) => {
                let content = if name == &self.stdout_tab.name {
                    Some(&self.stdout_tab.content)
                } else if name == &self.stderr_tab.name {
                    Some(&self.stderr_tab.content)
                } else if name == &self.stdin_tab.name {
                    Some(&self.stdin_tab.content)
                } else if name == &self.fileio_tab.name {
                    Some(&self.fileio_tab.content)
                } else {
                    None
                };

                if let Some(lines) = content {
                    let prefix = if self.flow_name.is_empty() {
                        String::new()
                    } else {
                        format!("{}_", self.flow_name)
                    };
                    let dialog = rfd::FileDialog::new()
                        .add_filter("Text", &["txt"])
                        .set_file_name(format!("{prefix}{name}.txt"));
                    if let Some(path) = dialog.save_file() {
                        if let Err(e) = fs::write(&path, lines.join("\n")) {
                            let msg = format!("Failed to save {name}: {e}");
                            error!("{msg}");
                            return Task::done(Message::SaveError(msg));
                        }
                    }
                }
            }
            Message::SaveImage(ref name) => {
                if let Some(image_ref) = self.images_tab.images.get(name) {
                    let dialog = rfd::FileDialog::new()
                        .add_filter("PNG", &["png"])
                        .set_file_name(name);
                    if let Some(path) = dialog.save_file() {
                        if let Err(e) = image_ref
                            .data
                            .save_with_format(&path, image::ImageFormat::Png)
                        {
                            let msg = format!("Failed to save image {name}: {e}");
                            error!("{msg}");
                            return Task::done(Message::SaveError(msg));
                        }
                    }
                }
            }
            _ => {}
        }

        Task::none()
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        Tabs::new(Message::TabSelected)
            .push(0, self.stdout_tab.tab_label(), self.stdout_tab.view())
            .push(1, self.stderr_tab.tab_label(), self.stderr_tab.view())
            .push(2, self.stdin_tab.tab_label(), self.stdin_tab.view())
            .push(3, self.images_tab.tab_label(), self.images_tab.view())
            .push(4, self.fileio_tab.tab_label(), self.fileio_tab.view())
            .set_active_tab(&self.active_tab)
            .into()
    }

    pub(crate) fn clear(&mut self) {
        self.stdout_tab.clear();
        self.stderr_tab.clear();
        self.stdin_tab.clear();
        self.images_tab.clear();
        self.fileio_tab.clear();
    }
}

pub trait Tab {
    type Message;

    fn tab_label(&self) -> TabLabel;

    fn view(&self) -> Element<'_, Self::Message>;

    fn clear(&mut self);
}

pub(crate) struct StdOutTab {
    pub name: String,
    pub id: Id,
    pub content: Vec<String>,
    pub auto_scroll: bool,
    pub unread_count: usize,
}

impl Tab for StdOutTab {
    type Message = Message;

    fn tab_label(&self) -> TabLabel {
        if self.unread_count > 0 {
            TabLabel::Text(format!("{} ({})", self.name, self.unread_count))
        } else {
            TabLabel::Text(self.name.clone())
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let text_column =
            Column::with_children(self.content.iter().cloned().map(text).map(Element::from))
                .width(Length::Fill)
                .padding(1);

        let scrollable = Scrollable::new(text_column)
            .height(Length::Fill)
            .id(self.id.clone());

        let toggler = toggler(self.auto_scroll)
            .label(format!("Auto-scroll {}", self.name))
            .on_toggle(|v| Message::StdioAutoScrollTogglerChanged(self.id.clone(), v))
            .width(Length::Shrink);

        let save_button =
            Button::new(Text::new("Save")).on_press(Message::SaveTabContent(self.name.clone()));
        let clear_button =
            Button::new(Text::new("Clear")).on_press(Message::ClearTab(self.name.clone()));

        let toolbar = Row::new()
            .push(toggler)
            .push(save_button)
            .push(clear_button)
            .spacing(10);

        Column::new().push(toolbar).push(scrollable).into()
    }

    fn clear(&mut self) {
        self.content.clear();
        self.unread_count = 0;
    }
}

pub(crate) struct ImageTab {
    name: String,
    pub images: HashMap<String, ImageReference>,
    pub new_activity: bool,
}

impl ImageTab {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            images: HashMap::default(),
            new_activity: false,
        }
    }
}

impl ImageTab {
    const MIN_DISPLAY_WIDTH: u32 = 400;
}

fn scale_image(image_ref: &ImageReference, min_width: u32) -> (u32, u32, Vec<u8>) {
    let scale = min_width.checked_div(image_ref.width).unwrap_or(1).max(1);

    let new_width = image_ref.width * scale;
    let new_height = image_ref.height * scale;
    let mut scaled = Vec::with_capacity((new_width * new_height * 4) as usize);

    for y in 0..image_ref.height {
        let row: Vec<u8> = (0..image_ref.width)
            .flat_map(|x| {
                let pixel = image_ref.data.get_pixel(x, y).0;
                pixel
                    .iter()
                    .copied()
                    .cycle()
                    .take(4 * scale as usize)
                    .collect::<Vec<u8>>()
            })
            .collect();
        for _ in 0..scale {
            scaled.extend_from_slice(&row);
        }
    }

    (new_width, new_height, scaled)
}

impl Tab for ImageTab {
    type Message = Message;

    fn tab_label(&self) -> TabLabel {
        if self.new_activity {
            TabLabel::Text(format!("{} *", self.name))
        } else {
            TabLabel::Text(self.name.clone())
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let mut col = Column::new().spacing(10);

        if self.images.is_empty() {
            col = col.push(Text::new("No images yet"));
        }

        for (name, image_ref) in &self.images {
            let (display_width, display_height, display_data) =
                scale_image(image_ref, Self::MIN_DISPLAY_WIDTH);
            let handle = Handle::from_rgba(display_width, display_height, display_data);
            let viewer = Viewer::new(handle)
                .filter_method(FilterMethod::Nearest)
                .content_fit(ContentFit::ScaleDown)
                .min_scale(0.1)
                .max_scale(10.0)
                .width(Length::Fill)
                .height(Length::Fixed(f32::from(
                    u16::try_from(display_height.min(300)).unwrap_or(300),
                )));
            let label = format!("{name} ({}x{})", image_ref.width, image_ref.height);
            let save_button =
                Button::new(Text::new("Save")).on_press(Message::SaveImage(name.clone()));
            let header = Row::new()
                .push(Text::new(label))
                .push(save_button)
                .spacing(10);
            col = col.push(header).push(viewer);
        }

        Scrollable::new(col)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn clear(&mut self) {
        self.images = HashMap::default();
    }
}

pub(crate) struct StdInTab {
    pub name: String,
    pub id: Id,
    pub content: Vec<String>,
    pub cursor: usize,
    pub text: String,
    pub eof_signaled: bool,
    pub waiting_for_input: bool,
}

impl StdInTab {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            id: Lazy::new(Id::unique).clone(),
            content: vec![],
            cursor: 0,
            text: String::new(),
            eof_signaled: false,
            waiting_for_input: false,
        }
    }

    /// New text has been typed into the STDIN text box
    pub fn text_entered(&mut self, text: String) {
        self.text = text;
    }

    /// A new line of text for standard input has been sent
    pub fn new_line(&mut self, line: String) {
        self.content.push(line);
        self.text = String::new();
    }

    /// return the next available line of standard input, or EOF
    pub fn get_line(&mut self) -> Option<String> {
        if let Some(line) = self.content.get(self.cursor) {
            self.cursor += 1;
            Some(line.clone())
        } else {
            None
        }
    }

    /// return all available standard input between the cursor and the end of content
    pub fn get_all(&mut self) -> Option<String> {
        if self.content.len() > self.cursor {
            let mut buf = String::new();
            for line in self.cursor..self.content.len() {
                if let Some(line) = self.content.get(line) {
                    buf.push_str(line);
                }
            }
            self.cursor = self.content.len();
            Some(buf)
        } else {
            // advanced beyond the available text!
            None
        }
    }
}

impl Tab for StdInTab {
    type Message = Message;

    fn tab_label(&self) -> TabLabel {
        if self.waiting_for_input {
            TabLabel::Text(format!("{} (waiting)", self.name))
        } else {
            TabLabel::Text(self.name.clone())
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let text_column =
            Column::with_children(self.content.iter().cloned().map(text).map(Element::from))
                .width(Length::Fill)
                .padding(1);

        let save_button =
            Button::new(Text::new("Save")).on_press(Message::SaveTabContent(self.name.clone()));
        let toolbar = Row::new().push(save_button).spacing(10);

        let text_input = TextInput::new("Enter new line of Standard input", &self.text)
            .on_input(Message::NewStdin)
            .on_paste(Message::NewStdin)
            .on_submit(Message::LineOfStdin(self.text.clone()))
            .width(Length::Fill)
            .padding(10);
        let eof_button = Button::new(Text::new("EOF")).on_press(Message::SendEof);
        let input_row = Row::new().push(text_input).push(eof_button).spacing(5);
        let scrollable = Scrollable::new(text_column)
            .height(Length::Fill)
            .id(self.id.clone());

        Column::new()
            .push(toolbar)
            .push(scrollable)
            .push(input_row)
            .into()
    }

    // Avoid clearing standard input - to allow the user to type in input ahead of the
    // flow being run
    fn clear(&mut self) {}
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use super::*;

    #[test]
    fn stdin_new() {
        let tab = StdInTab::new("test");
        assert_eq!(tab.name, "test");
        assert!(tab.content.is_empty());
        assert_eq!(tab.cursor, 0);
        assert!(tab.text.is_empty());
        assert!(!tab.eof_signaled);
    }

    #[test]
    fn stdin_text_entered() {
        let mut tab = StdInTab::new("test");
        tab.text_entered("hello".into());
        assert_eq!(tab.text, "hello");
    }

    #[test]
    fn stdin_new_line() {
        let mut tab = StdInTab::new("test");
        tab.text_entered("typing".into());
        tab.new_line("first line".into());
        assert_eq!(tab.content, vec!["first line"]);
        assert!(tab.text.is_empty()); // text cleared after new_line
    }

    #[test]
    fn stdin_get_line_returns_lines_in_order() {
        let mut tab = StdInTab::new("test");
        tab.new_line("line1".into());
        tab.new_line("line2".into());

        assert_eq!(tab.get_line(), Some("line1".into()));
        assert_eq!(tab.get_line(), Some("line2".into()));
        assert_eq!(tab.get_line(), None); // EOF
    }

    #[test]
    fn stdin_get_line_returns_raw_input() {
        let mut tab = StdInTab::new("test");
        tab.new_line("world".into());

        assert_eq!(tab.get_line(), Some("world".into()));
    }

    #[test]
    fn stdin_get_line_eof_when_empty() {
        let mut tab = StdInTab::new("test");
        assert_eq!(tab.get_line(), None);
    }

    #[test]
    fn stdin_get_all_returns_all_content() {
        let mut tab = StdInTab::new("test");
        tab.new_line("a".into());
        tab.new_line("b".into());
        tab.new_line("c".into());

        assert_eq!(tab.get_all(), Some("abc".into()));
        assert_eq!(tab.get_all(), None); // cursor advanced past end
    }

    #[test]
    fn stdin_get_all_after_partial_get_line() {
        let mut tab = StdInTab::new("test");
        tab.new_line("a".into());
        tab.new_line("b".into());
        tab.new_line("c".into());

        assert_eq!(tab.get_line(), Some("a".into())); // cursor at 1
        assert_eq!(tab.get_all(), Some("bc".into())); // gets remaining
        assert_eq!(tab.get_all(), None); // EOF
    }

    #[test]
    fn stdin_get_all_eof_when_empty() {
        let mut tab = StdInTab::new("test");
        assert_eq!(tab.get_all(), None);
    }

    #[test]
    fn stdin_clear_does_not_clear() {
        let mut tab = StdInTab::new("test");
        tab.new_line("preserved".into());
        Tab::clear(&mut tab);
        assert_eq!(tab.content, vec!["preserved"]); // stdin clear is intentionally a no-op
    }

    #[test]
    fn stdout_clear() {
        let mut tab = StdOutTab {
            name: "test".into(),
            id: Id::unique(),
            content: vec!["line1".into(), "line2".into()],
            auto_scroll: true,
            unread_count: 0,
        };
        Tab::clear(&mut tab);
        assert!(tab.content.is_empty());
    }

    fn tab_label_text(label: TabLabel) -> String {
        match label {
            TabLabel::Text(s) => s,
            _ => panic!("Expected TabLabel::Text"),
        }
    }

    #[test]
    fn stdout_tab_label_no_unread() {
        let tab = StdOutTab {
            name: "Stdout".into(),
            id: Id::unique(),
            content: vec![],
            auto_scroll: true,
            unread_count: 0,
        };
        assert_eq!(tab_label_text(tab.tab_label()), "Stdout");
    }

    #[test]
    fn stdout_tab_label_with_unread() {
        let tab = StdOutTab {
            name: "Stdout".into(),
            id: Id::unique(),
            content: vec!["line".into()],
            auto_scroll: true,
            unread_count: 3,
        };
        assert_eq!(tab_label_text(tab.tab_label()), "Stdout (3)");
    }

    #[test]
    fn stdout_clear_resets_unread() {
        let mut tab = StdOutTab {
            name: "test".into(),
            id: Id::unique(),
            content: vec!["line".into()],
            auto_scroll: true,
            unread_count: 5,
        };
        Tab::clear(&mut tab);
        assert!(tab.content.is_empty());
        assert_eq!(tab.unread_count, 0);
    }

    #[test]
    fn image_tab_label_no_activity() {
        let tab = ImageTab::new("Images");
        assert_eq!(tab_label_text(tab.tab_label()), "Images");
    }

    #[test]
    fn image_tab_label_with_activity() {
        let mut tab = ImageTab::new("Images");
        tab.new_activity = true;
        assert_eq!(tab_label_text(tab.tab_label()), "Images *");
    }

    #[test]
    fn stdin_tab_label_not_waiting() {
        let tab = StdInTab::new("Stdin");
        assert_eq!(tab_label_text(tab.tab_label()), "Stdin");
    }

    #[test]
    fn stdin_tab_label_waiting() {
        let mut tab = StdInTab::new("Stdin");
        tab.waiting_for_input = true;
        assert_eq!(tab_label_text(tab.tab_label()), "Stdin (waiting)");
    }

    #[test]
    fn tab_select_resets_unread() {
        let mut tabs = TabSet::new();
        tabs.stdout_tab.unread_count = 5;
        tabs.stderr_tab.unread_count = 3;
        drop(tabs.update(Message::TabSelected(0)));
        assert_eq!(tabs.stdout_tab.unread_count, 0);
        assert_eq!(tabs.stderr_tab.unread_count, 3);
    }

    #[test]
    fn tabset_flow_name_default_empty() {
        let tabs = TabSet::new();
        assert!(tabs.flow_name.is_empty());
    }

    #[test]
    fn tabset_clear_resets_all_tabs() {
        let mut tabs = TabSet::new();
        tabs.stdout_tab.content.push("hello".into());
        tabs.stderr_tab.content.push("error".into());
        tabs.fileio_tab.content.push("file".into());
        tabs.flow_name = "myflow".into();
        tabs.clear();
        assert!(tabs.stdout_tab.content.is_empty());
        assert!(tabs.stderr_tab.content.is_empty());
        assert!(tabs.fileio_tab.content.is_empty());
    }

    #[test]
    fn clear_tab_stdout() {
        let mut tabs = TabSet::new();
        tabs.stdout_tab.content.push("line1".into());
        tabs.stdout_tab.content.push("line2".into());
        drop(tabs.update(Message::ClearTab("Stdout".into())));
        assert!(tabs.stdout_tab.content.is_empty());
    }

    #[test]
    fn clear_tab_stderr() {
        let mut tabs = TabSet::new();
        tabs.stderr_tab.content.push("err".into());
        drop(tabs.update(Message::ClearTab("Stderr".into())));
        assert!(tabs.stderr_tab.content.is_empty());
    }

    #[test]
    fn clear_tab_unknown_is_noop() {
        let mut tabs = TabSet::new();
        tabs.stdout_tab.content.push("keep".into());
        drop(tabs.update(Message::ClearTab("Unknown".into())));
        assert_eq!(tabs.stdout_tab.content, vec!["keep"]);
    }

    #[test]
    fn save_image_unknown_name_is_noop() {
        let mut tabs = TabSet::new();
        drop(tabs.update(Message::SaveImage("nonexistent".into())));
    }

    #[test]
    fn save_tab_content_unknown_name_is_noop() {
        let mut tabs = TabSet::new();
        tabs.stdout_tab.content.push("data".into());
        drop(tabs.update(Message::SaveTabContent("Unknown".into())));
        assert_eq!(tabs.stdout_tab.content, vec!["data"]);
    }

    #[test]
    fn image_tab_clear_removes_images() {
        let mut tab = ImageTab::new("Images");
        tab.images.insert(
            "test".into(),
            ImageReference {
                width: 1,
                height: 1,
                data: image::RgbaImage::new(1, 1),
            },
        );
        tab.new_activity = true;
        Tab::clear(&mut tab);
        assert!(tab.images.is_empty());
    }
}
