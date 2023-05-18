use iced::{
    Application, Command, Element, Length, Rectangle, Settings, Theme,
};
use iced::executor;
use iced::widget::{canvas, container};
use iced::widget::canvas::{Cursor, Geometry};

pub fn main() -> iced::Result {
    FlowIde::run(Settings {
        antialiasing: true,
        ..Settings::default()
    })
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum Message {
    Start,
}

struct FlowIde {
}

impl Application for FlowIde {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            FlowIde {},
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("FlowIde")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Start => {}
        }

        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let canvas = canvas(self as &Self)
            .width(Length::Fill)
            .height(Length::Fill);

        container(canvas)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .into()
    }
}

impl<Message> canvas::Program<Message> for FlowIde {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        _theme: &Theme,
        _bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        vec![]
    }
}
