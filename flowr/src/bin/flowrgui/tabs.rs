use iced::{Element, Length};
use iced::widget::{Column, text, toggler};
use iced::widget::image::{Handle, Viewer};
use iced::widget::scrollable::{Id, Scrollable};
use iced_aw::{TabLabel, Tabs};
use once_cell::sync::Lazy;

use crate::{ImageReference, Message};

pub(crate) struct TabSet {
    pub active_tab: usize,
    pub stdout_tab: StdIOTab,
    pub stderr_tab: StdIOTab,
    pub images_tab: ImageTab,
}

impl TabSet {
    pub(crate) fn new() -> Self {
        TabSet {
            active_tab: 0,
            stdout_tab: StdIOTab {
                name: "Stdout".to_owned(),
                id: Lazy::new(Id::unique).clone(),
                content: vec!(),
                auto_scroll: true
            },
            stderr_tab: StdIOTab {
                name: "Stderr".to_owned(),
                id: Lazy::new(Id::unique).clone(),
                content: vec!(),
                auto_scroll: true
            },
            images_tab: ImageTab {
                name: "Images".to_owned(),
                image: None,
            }
        }
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        Tabs::new(Message::TabSelected)
            .push(0, self.stdout_tab.tab_label(), self.stdout_tab.view())
            .push(1, self.stderr_tab.tab_label(), self.stderr_tab.view())
            .push(2, self.images_tab.tab_label(), self.images_tab.view())
            .set_active_tab(&self.active_tab)
            .into()
    }

    pub(crate) fn clear(&mut self) {
        self.stdout_tab.clear();
        self.stderr_tab.clear();
        // TODO clear images and others
    }

}

pub trait Tab {
    type Message;

    fn title(&self) -> String;

    fn tab_label(&self) -> TabLabel;

    fn view(&self) -> Element<'_, Self::Message>;

    fn clear(&mut self);
}

pub(crate) struct StdIOTab {
    pub name: String,
    pub id: Id,
    pub content: Vec<String>,
    pub auto_scroll: bool,
}

impl Tab for StdIOTab {
    type Message = Message;

    fn title(&self) -> String {
        String::from(&self.name)
    }

    fn tab_label(&self) -> TabLabel {
        TabLabel::Text(self.name.to_string())
    }

    fn view(&self) -> Element<Message> {
        let text_column = Column::with_children(
            self.content
                .iter()
                .cloned()
                .map(text)
                .map(Element::from)
                .collect(),
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
    pub image: Option<ImageReference>,
}
impl Tab for ImageTab {
    type Message = Message;

    fn title(&self) -> String {
        String::from(&self.name)
    }

    fn tab_label(&self) -> TabLabel {
        TabLabel::Text(self.name.to_string())
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let mut col = Column::new();

        // TODO add a scrollable row of images in a Tab
        if let Some(ImageReference { name: _, width, height, data}) = &self.image {
            col = col.push(Viewer::new(
                Handle::from_pixels( *width, *height, data.as_raw().clone())));
            // TODO switch to the images tab when image first written to
        }

        col.into()
    }

    fn clear(&mut self) {
        todo!()
    }
}