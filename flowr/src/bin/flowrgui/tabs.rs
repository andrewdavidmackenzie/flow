use std::collections::HashMap;
use std::fs;

use iced::widget::image::{FilterMethod, Handle, Viewer};
use iced::widget::operation::{self, RelativeOffset};
use iced::widget::scrollable::Scrollable;
use iced::widget::TextInput;
use iced::widget::{text, toggler, Button, Column, Container, Id, Row, Text};
use iced::{Background, Border, ContentFit, Element, Length, Task};
use log::error;
use once_cell::sync::Lazy;

#[cfg(feature = "debugger")]
use crate::DebugEventLine;
use crate::{ImageReference, Message};

#[allow(clippy::struct_field_names)]
pub(crate) struct TabSet {
    pub active_tab: usize,
    pub stdout_tab: StdOutTab,
    pub stderr_tab: StdOutTab,
    pub stdin_tab: StdInTab,
    pub images_tab: ImageTab,
    pub fileio_tab: StdOutTab,
    #[cfg(feature = "debugger")]
    pub debug_tab: DebugTab,
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
            #[cfg(feature = "debugger")]
            debug_tab: DebugTab::new("Debug"),
            flow_name: String::new(),
        }
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TabSelected(tab_index) => {
                self.active_tab = tab_index;
                match tab_index {
                    0 if self.stdout_tab.auto_scroll => self.stdout_tab.unread_count = 0,
                    1 if self.stderr_tab.auto_scroll => self.stderr_tab.unread_count = 0,
                    3 => self.images_tab.new_activity = false,
                    4 if self.fileio_tab.auto_scroll => self.fileio_tab.unread_count = 0,
                    #[cfg(feature = "debugger")]
                    5 if self.debug_tab.auto_scroll => self.debug_tab.unread_count = 0,
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
                #[cfg(feature = "debugger")]
                if name == &self.debug_tab.name {
                    self.debug_tab.clear();
                }
            }
            Message::StdioAutoScrollTogglerChanged(id, value) => {
                if id == self.stdout_tab.id {
                    self.stdout_tab.auto_scroll = value;
                }
                #[cfg(feature = "debugger")]
                if id == self.debug_tab.id {
                    self.debug_tab.auto_scroll = value;
                }
                if id == self.stderr_tab.id {
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
                #[cfg(feature = "debugger")]
                if name == &self.debug_tab.name {
                    let debug_lines: Vec<String> = self
                        .debug_tab
                        .content
                        .iter()
                        .map(|l| l.text.clone())
                        .collect();
                    let prefix = if self.flow_name.is_empty() {
                        String::new()
                    } else {
                        format!("{}_", self.flow_name)
                    };
                    let dialog = rfd::FileDialog::new()
                        .add_filter("Text", &["txt"])
                        .set_file_name(format!("{prefix}{name}.txt"));
                    if let Some(path) = dialog.save_file() {
                        if let Err(e) = fs::write(&path, debug_lines.join("\n")) {
                            let msg = format!("Failed to save {name}: {e}");
                            error!("{msg}");
                            return Task::done(Message::SaveError(msg));
                        }
                    }
                    return Task::none();
                }

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
        let mut tab_labels: Vec<(usize, String)> = vec![
            (0, self.stdout_tab.label_text()),
            (1, self.stderr_tab.label_text()),
            (2, self.stdin_tab.label_text()),
            (3, self.images_tab.label_text()),
            (4, self.fileio_tab.label_text()),
        ];
        #[cfg(feature = "debugger")]
        tab_labels.push((5, self.debug_tab.label_text()));

        let tab_bar = Row::with_children(
            tab_labels
                .into_iter()
                .map(|(idx, label)| {
                    let is_active = self.active_tab == idx;
                    let btn = Button::new(Text::new(label).size(crate::theme::FONT_MD))
                        .padding([6, 16])
                        .on_press(Message::TabSelected(idx))
                        .style(move |theme: &iced::Theme, status| {
                            tab_button_style(theme, status, is_active)
                        });
                    Element::from(btn)
                })
                .collect::<Vec<Element<'_, Message>>>(),
        );

        let content: Element<'_, Message> = match self.active_tab {
            0 => self.stdout_tab.view(),
            1 => self.stderr_tab.view(),
            2 => self.stdin_tab.view(),
            3 => self.images_tab.view(),
            4 => self.fileio_tab.view(),
            #[cfg(feature = "debugger")]
            5 => self.debug_tab.view(),
            _ => text("").into(),
        };

        let content_panel = Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|theme: &iced::Theme| {
                let palette = theme.palette();
                iced::widget::container::Style {
                    background: Some(Background::Color(iced::Color {
                        a: 0.08,
                        ..palette.text
                    })),
                    border: Border {
                        color: iced::Color::TRANSPARENT,
                        width: 0.0,
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                }
            });

        Column::new()
            .spacing(0)
            .push(tab_bar)
            .push(content_panel)
            .into()
    }

    pub(crate) fn clear(&mut self) {
        self.stdout_tab.clear();
        self.stderr_tab.clear();
        self.stdin_tab.clear();
        self.images_tab.clear();
        self.fileio_tab.clear();
        #[cfg(feature = "debugger")]
        self.debug_tab.clear();
    }
}

pub trait Tab {
    type Message;

    fn label_text(&self) -> String;

    fn view(&self) -> Element<'_, Self::Message>;

    fn clear(&mut self);
}

fn tab_button_style(
    theme: &iced::Theme,
    status: iced::widget::button::Status,
    is_active: bool,
) -> iced::widget::button::Style {
    let palette = theme.palette();
    let active_bg = iced::Color {
        a: 0.08,
        ..palette.text
    };
    let top_only = iced::border::Radius {
        top_left: 4.0,
        top_right: 4.0,
        bottom_right: 0.0,
        bottom_left: 0.0,
    };
    let base = iced::widget::button::Style {
        border: Border {
            radius: top_only,
            width: 0.0,
            color: iced::Color::TRANSPARENT,
        },
        ..Default::default()
    };

    if is_active {
        iced::widget::button::Style {
            background: Some(Background::Color(active_bg)),
            text_color: palette.text,
            ..base
        }
    } else {
        match status {
            iced::widget::button::Status::Hovered => iced::widget::button::Style {
                background: Some(Background::Color(iced::Color {
                    a: 0.03,
                    ..palette.text
                })),
                text_color: palette.text,
                ..base
            },
            _ => iced::widget::button::Style {
                background: None,
                text_color: iced::Color {
                    a: 0.6,
                    ..palette.text
                },
                ..base
            },
        }
    }
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

    fn label_text(&self) -> String {
        if self.unread_count > 0 {
            format!("{} ({})", self.name, self.unread_count)
        } else {
            self.name.clone()
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let text_column = Column::with_children(
            self.content
                .iter()
                .cloned()
                .map(|s| text(s).shaping(iced::widget::text::Shaping::Advanced))
                .map(Element::from),
        )
        .width(Length::Fill)
        .padding(1);

        let scrollable = Scrollable::new(text_column)
            .height(Length::Fill)
            .id(self.id.clone());

        let toggler = toggler(self.auto_scroll)
            .label(format!("Auto-scroll {}", self.name))
            .on_toggle(|v| Message::StdioAutoScrollTogglerChanged(self.id.clone(), v))
            .size(14.0)
            .text_size(crate::theme::FONT_MD)
            .width(Length::Shrink);

        let save_button = Button::new(Text::new("Save").size(crate::theme::FONT_MD))
            .on_press(Message::SaveTabContent(self.name.clone()))
            .style(crate::theme::list_button)
            .padding(crate::theme::BUTTON_PAD_SM);
        let clear_button = Button::new(Text::new("Clear").size(crate::theme::FONT_MD))
            .on_press(Message::ClearTab(self.name.clone()))
            .style(crate::theme::list_button)
            .padding(crate::theme::BUTTON_PAD_SM);

        let toolbar = Row::new()
            .push(toggler)
            .push(save_button)
            .push(clear_button)
            .spacing(crate::theme::SPACE_MD)
            .padding(crate::theme::SPACE_XS)
            .align_y(iced::alignment::Vertical::Center);

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

    fn label_text(&self) -> String {
        if self.new_activity {
            format!("{} *", self.name)
        } else {
            self.name.clone()
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
            let save_button = Button::new(Text::new("Save"))
                .on_press(Message::SaveImage(name.clone()))
                .style(crate::theme::styled_button)
                .padding([4, 12]);
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

    fn label_text(&self) -> String {
        if self.waiting_for_input {
            format!("{} (waiting)", self.name)
        } else {
            self.name.clone()
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let text_column =
            Column::with_children(self.content.iter().cloned().map(text).map(Element::from))
                .width(Length::Fill)
                .padding(1);

        let save_button = Button::new(Text::new("Save"))
            .on_press(Message::SaveTabContent(self.name.clone()))
            .style(crate::theme::styled_button)
            .padding([4, 12]);
        let toolbar = Row::new().push(save_button).spacing(10).padding(4);

        let text_input = TextInput::new("Enter new line of Standard input", &self.text)
            .on_input(Message::NewStdin)
            .on_paste(Message::NewStdin)
            .on_submit(Message::LineOfStdin(self.text.clone()))
            .width(Length::Fill)
            .padding(10);
        let eof_button = Button::new(Text::new("EOF"))
            .on_press(Message::SendEof)
            .style(crate::theme::styled_button)
            .padding([4, 12]);
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

#[cfg(feature = "debugger")]
pub(crate) struct DebugTab {
    pub name: String,
    pub id: Id,
    pub content: Vec<DebugEventLine>,
    pub auto_scroll: bool,
    pub unread_count: usize,
    next_section_id: usize,
    current_section_id: usize,
    collapsed: std::collections::HashSet<usize>,
}

#[cfg(feature = "debugger")]
impl DebugTab {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            id: Lazy::new(Id::unique).clone(),
            content: Vec::new(),
            auto_scroll: true,
            unread_count: 0,
            next_section_id: 1,
            current_section_id: 0,
            collapsed: std::collections::HashSet::new(),
        }
    }

    pub fn push(&mut self, mut line: DebugEventLine) {
        if line.separator {
            self.current_section_id = self.next_section_id;
            self.next_section_id += 1;
            line.section_id = self.current_section_id;
        } else {
            line.section_id = self.current_section_id;
        }
        self.content.push(line);
    }

    pub fn push_text(&mut self, text: String) {
        self.push(DebugEventLine {
            text,
            color: None,
            separator: false,
            links: Vec::new(),
            section_id: 0,
        });
    }

    pub fn toggle_section(&mut self, section_id: usize) {
        if self.collapsed.contains(&section_id) {
            self.collapsed.remove(&section_id);
        } else {
            self.collapsed.insert(section_id);
        }
    }
}

#[cfg(feature = "debugger")]
impl Tab for DebugTab {
    type Message = Message;

    fn label_text(&self) -> String {
        if self.unread_count > 0 {
            format!("{} ({})", self.name, self.unread_count)
        } else {
            self.name.clone()
        }
    }

    #[allow(clippy::too_many_lines)]
    fn view(&self) -> Element<'_, Message> {
        let text_column = Column::with_children(
            self.content
                .iter()
                .filter(|line| line.separator || !self.collapsed.contains(&line.section_id))
                .map(|line| {
                    if line.separator {
                        let color = line.color.unwrap_or(iced::Color::WHITE);
                        let is_collapsed = self.collapsed.contains(&line.section_id);
                        let indicator = if is_collapsed { "\u{25B6}" } else { "\u{25BC}" };
                        let section_id = line.section_id;
                        let toggle_btn = Button::new(
                            Text::new(indicator)
                                .size(14)
                                .shaping(iced::widget::text::Shaping::Advanced),
                        )
                        .on_press(Message::DebugToggleSection(section_id))
                        .style(crate::theme::list_button)
                        .padding([2, 6]);
                        let rule_left = iced::widget::rule::horizontal(1);
                        let rule_right = iced::widget::rule::horizontal(1);
                        let label = text(line.text.clone())
                            .shaping(iced::widget::text::Shaping::Advanced)
                            .size(13)
                            .color(color);
                        Element::from(
                            Row::new()
                                .align_y(iced::alignment::Vertical::Center)
                                .spacing(6)
                                .push(toggle_btn)
                                .push(rule_left)
                                .push(label)
                                .push(rule_right)
                                .padding([4, 0]),
                        )
                    } else if line.links.is_empty() {
                        let mut t =
                            text(line.text.clone()).shaping(iced::widget::text::Shaping::Advanced);
                        if let Some(color) = line.color {
                            t = t.color(color);
                        }
                        Element::from(t)
                    } else {
                        let base_color = line.color;
                        let link_color = crate::theme::TEXT_LINK;
                        let mut spans: Vec<iced::widget::text::Span<'_, String>> = Vec::new();
                        let mut pos = 0;
                        for link in &line.links {
                            if link.start > pos {
                                let mut s =
                                    iced::widget::span(line.text[pos..link.start].to_string());
                                if let Some(c) = base_color {
                                    s = s.color(c);
                                }
                                spans.push(s);
                            }
                            spans.push(
                                iced::widget::span(line.text[link.start..link.end].to_string())
                                    .color(link_color)
                                    .underline(true)
                                    .link(link.spec.clone()),
                            );
                            pos = link.end;
                        }
                        if pos < line.text.len() {
                            let mut s = iced::widget::span(line.text[pos..].to_string());
                            if let Some(c) = base_color {
                                s = s.color(c);
                            }
                            spans.push(s);
                        }
                        Element::from(
                            iced::widget::rich_text(spans).on_link_click(Message::DebugInspectLink),
                        )
                    }
                }),
        )
        .width(Length::Fill)
        .padding(1);

        let scrollable = Scrollable::new(text_column)
            .height(Length::Fill)
            .id(self.id.clone());

        let toggler = toggler(self.auto_scroll)
            .label(format!("Auto-scroll {}", self.name))
            .on_toggle(|v| Message::StdioAutoScrollTogglerChanged(self.id.clone(), v))
            .size(14.0)
            .text_size(crate::theme::FONT_MD)
            .width(Length::Shrink);

        let save_button = Button::new(Text::new("Save").size(crate::theme::FONT_MD))
            .on_press(Message::SaveTabContent(self.name.clone()))
            .style(crate::theme::list_button)
            .padding(crate::theme::BUTTON_PAD_SM);
        let clear_button = Button::new(Text::new("Clear").size(crate::theme::FONT_MD))
            .on_press(Message::ClearTab(self.name.clone()))
            .style(crate::theme::list_button)
            .padding(crate::theme::BUTTON_PAD_SM);

        let toolbar = Row::new()
            .push(toggler)
            .push(save_button)
            .push(clear_button)
            .spacing(crate::theme::SPACE_MD)
            .padding(crate::theme::SPACE_XS)
            .align_y(iced::alignment::Vertical::Center);

        Column::new().push(toolbar).push(scrollable).into()
    }

    fn clear(&mut self) {
        self.content.clear();
        self.unread_count = 0;
        self.collapsed.clear();
        self.next_section_id = 1;
        self.current_section_id = 0;
    }
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

    #[test]
    fn stdout_tab_label_no_unread() {
        let tab = StdOutTab {
            name: "Stdout".into(),
            id: Id::unique(),
            content: vec![],
            auto_scroll: true,
            unread_count: 0,
        };
        assert_eq!(tab.label_text(), "Stdout");
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
        assert_eq!(tab.label_text(), "Stdout (3)");
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
        assert_eq!(tab.label_text(), "Images");
    }

    #[test]
    fn image_tab_label_with_activity() {
        let mut tab = ImageTab::new("Images");
        tab.new_activity = true;
        assert_eq!(tab.label_text(), "Images *");
    }

    #[test]
    fn stdin_tab_label_not_waiting() {
        let tab = StdInTab::new("Stdin");
        assert_eq!(tab.label_text(), "Stdin");
    }

    #[test]
    fn stdin_tab_label_waiting() {
        let mut tab = StdInTab::new("Stdin");
        tab.waiting_for_input = true;
        assert_eq!(tab.label_text(), "Stdin (waiting)");
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
