use std::{path::PathBuf, sync::Arc};

use iced::widget::{column, container, scrollable, text, vertical_space};
use iced::{Alignment, Command, Element, Length};

use crate::stellaris::{parse_stellaris, Stellaris};

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Arc<Stellaris>),
}

#[derive(Debug)]
pub struct StellarisView {
    stellaris: Stellaris,
    is_loading: bool,
}

impl StellarisView {
    pub fn new(path: PathBuf) -> (Self, iced::Command<Message>) {
        (
            StellarisView {
                stellaris: Stellaris::default(),
                is_loading: true,
            },
            Command::perform(parse(path.clone()), Message::Loaded),
        )
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Loaded(stellaris) => {
                self.is_loading = false;
                self.stellaris = (*stellaris).clone();

                Command::none()
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        if self.is_loading {
            return container(
                column![text("Loading...").size(50), vertical_space().height(50),]
                    .width(Length::Fill)
                    .align_items(Alignment::Center)
                    .spacing(10),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into();
        }

        let variables = if self.stellaris.variables.is_empty() {
            column![]
        } else {
            column![
                text("Variables:").size(20),
                vertical_space().height(10),
                container(
                    scrollable(column![text(
                        self.stellaris
                            .variables
                            .iter()
                            .map(|(k, v)| format!("{}: {}", k, v))
                            .collect::<Vec<String>>()
                            .join("\n")
                    )])
                    .width(750)
                    .height(150)
                )
            ]
        };

        let view = column![variables,].spacing(20);

        let content = column![view]
            .width(Length::Fill)
            .align_items(Alignment::Center)
            .spacing(10);

        container(scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .padding(10)
            .into()
    }
}

async fn parse(path: PathBuf) -> Arc<Stellaris> {
    let stellaris = parse_stellaris(&path);
    Arc::new(stellaris)
}
