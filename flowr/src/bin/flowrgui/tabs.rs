use iced::{Element, Length};
use iced::widget::{Column, text, toggler};
use iced::widget::scrollable::{Id, Scrollable};
use iced_aw::TabLabel;

use crate::Message;

pub trait Tab {
    type Message;

    fn title(&self) -> String;

    fn tab_label(&self) -> TabLabel;

    fn view(&self) -> Element<'_, Self::Message>;

    fn clear(&mut self);
}

pub struct StdIOTab {
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