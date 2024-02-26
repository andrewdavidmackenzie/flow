use std::collections::HashMap;

use iced::{Command, Element, Length};
use iced::widget::{Column, scrollable, text, toggler};
use iced::widget::image::{Handle, Viewer};
use iced::widget::scrollable::{Id, Scrollable};
use iced::widget::TextInput;
use iced_aw::{TabBarStyles, TabLabel, Tabs};
use once_cell::sync::Lazy;

use crate::{ImageReference, Message};

pub(crate) struct TabSet {
    pub active_tab: usize,
    pub stdout_tab: StdOutTab,
    pub stderr_tab: StdOutTab,
    pub stdin_tab: StdInTab,
    pub images_tab: ImageTab,
    pub fileio_tab: StdOutTab,
}

impl TabSet {
    pub(crate) fn new() -> Self {
        TabSet {
            active_tab: 0,
            stdout_tab: StdOutTab {
                name: "Stdout".to_owned(),
                id: Lazy::new(Id::unique).clone(),
                content: vec!(),
                auto_scroll: true
            },
            stderr_tab: StdOutTab {
                name: "Stderr".to_owned(),
                id: Lazy::new(Id::unique).clone(),
                content: vec!(),
                auto_scroll: true
            },
            stdin_tab: StdInTab::new("Stdin"),
            images_tab: ImageTab::new("Images"),
            fileio_tab: StdOutTab {
                name: "FileIO".to_owned(),
                id: Lazy::new(Id::unique).clone(),
                content: vec!(),
                auto_scroll: true
            },
        }
    }

    pub(crate) fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::TabSelected(tab_index) => self.active_tab = tab_index,
            Message::StdioAutoScrollTogglerChanged(id, value) => {
                if id == self.stdout_tab.id {
                    self.stdout_tab.auto_scroll = value;
                }
                else {
                    self.stderr_tab.auto_scroll = value
                }

                if value {
                    return scrollable::snap_to(id,scrollable::RelativeOffset::END);
                }
            },
            _ => {},
        }

        Command::none()
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        Tabs::new(Message::TabSelected)
            .push(0, self.stdout_tab.tab_label(), self.stdout_tab.view())
            .push(1, self.stderr_tab.tab_label(), self.stderr_tab.view())
            .push(2, self.stdin_tab.tab_label(), self.stdin_tab.view())
            .push(3, self.images_tab.tab_label(), self.images_tab.view())
            .push(4, self.fileio_tab.tab_label(), self.fileio_tab.view())
            .set_active_tab(&self.active_tab)
            .tab_bar_style(TabBarStyles::Blue)
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
}

impl Tab for StdOutTab {
    type Message = Message;

    fn tab_label(&self) -> TabLabel {
        TabLabel::Text(self.name.to_string())
    }

    fn view(&self) -> Element<Message> {
        let text_column = Column::with_children(
            self.content
                .iter()
                .cloned()
                .map(text)
                .map(Element::from),
        )
            .width(Length::Fill)
            .padding(1);

        let scrollable = Scrollable::new(text_column)
            .height(Length::Fill)
            .id(self.id.clone());

        let toggler = toggler(
            format!("Auto-scroll {}", self.name),
            self.auto_scroll,
            |v| Message::StdioAutoScrollTogglerChanged(self.id.clone(), v))
            .width(Length::Shrink);

        Column::new()
            .push(toggler)
            .push(scrollable)
            .into()
    }

    fn clear(&mut self) {
        self.content.clear();
    }
}

pub(crate) struct ImageTab {
    name: String,
    pub images: HashMap<String, ImageReference>,
}

impl ImageTab {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            images: Default::default(),
        }
    }
}

impl Tab for ImageTab {
    type Message = Message;

    fn tab_label(&self) -> TabLabel {
        TabLabel::Text(self.name.to_string())
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let mut col = Column::new();

        for image_ref in self.images.values() {
            col = col.push(Viewer::new(
                Handle::from_pixels( image_ref.width, image_ref.height,
                                     image_ref.data.as_raw().clone())));
        }

        col.into()
    }

    fn clear(&mut self) {
        self.images = Default::default();
    }
}

pub(crate) struct StdInTab {
    pub name: String,
    pub id: Id,
    pub content: Vec<String>,
    pub cursor: usize,
    pub text: String,
}

impl StdInTab {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            id: Lazy::new(Id::unique).clone(),
            content: vec!(),
            cursor: 0,
            text: "".into(),
        }
    }

    /// New text has been typed into the STDIN text box
    pub fn text_entered(&mut self, text: String) {
        self.text = text;
    }

    /// A new line of text for standard input has been sent
    pub fn new_line(&mut self, line: String) {
        self.content.push(line);
        self.text = "".to_string();
    }

    /// return the next available line of standard input, or EOF
    pub fn get_line(&mut self, prompt: String) -> Option<String> {
        if let Some(line) = self.content.get_mut(self.cursor) {
            if !prompt.is_empty() {
                line.insert_str(0, &prompt);
            }
            self.cursor += 1;
            Some(line.to_string())
        } else {
            // advanced beyond the available text!
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
        TabLabel::Text(self.name.to_string())
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let text_column = Column::with_children(
            self.content
                .iter()
                .cloned()
                .map(text)
                .map(Element::from),
        )
            .width(Length::Fill)
            .padding(1);

        let text_input = TextInput::new(
            "Enter new line of Standard input", &self.text)
            .on_input(Message::NewStdin)
            .on_paste(Message::NewStdin)
            .on_submit(Message::LineOfStdin(self.text.clone()))
            .width(Length::Fill)
            .padding(10);
        let scrollable = Scrollable::new(text_column)
            .height(Length::Fill)
            .id(self.id.clone());

        Column::new()
            .push(scrollable)
            .push(text_input)
            .into()
    }

    // Avoid clearing standard input - to allow the user to type in input ahead of the
    // flow being run
    fn clear(&mut self) {}
}