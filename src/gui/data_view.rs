use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use iced::widget::{column, combo_box, container, scrollable, text, vertical_space};
use iced::{Alignment, Command, Element, Length};

use crate::game::parse_game;
use crate::parser::ConfigPair;

#[derive(Debug, Clone)]
pub enum Message {
    Selected(String),
    Loaded(Arc<HashMap<String, Vec<ConfigPair>>>),
}

#[derive(Debug)]
pub struct DataView {
    is_loading: bool,
    data: HashMap<String, Vec<ConfigPair>>,
    files: combo_box::State<String>,
    selected_file: Option<String>,
}

impl DataView {
    pub fn new(path: PathBuf) -> (Self, iced::Command<Message>) {
        (
            DataView {
                is_loading: true,
                data: HashMap::new(),
                files: combo_box::State::new(vec![]),
                selected_file: None,
            },
            Command::perform(parse(path.clone()), Message::Loaded),
        )
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Loaded(data) => {
                self.is_loading = false;
                self.data = (*data).clone();
                self.files = combo_box::State::new(self.data.keys().cloned().collect());

                Command::none()
            }
            Message::Selected(file) => {
                self.selected_file = Some(file);
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

        let combo_box = combo_box(
            &self.files,
            "Select a file",
            self.selected_file.as_ref(),
            Message::Selected,
        )
        .width(450);

        let selected_file = if let Some(file) = &self.selected_file {
            let file = self.data.get(file).unwrap();
            container(
                column![text(
                    file.iter()
                        .map(|v| format!("{}", v))
                        .collect::<Vec<String>>()
                        .join("\n")
                )]
                .spacing(20)
                .width(Length::Fill),
            )
        } else {
            container(
                column![text("Select a file to view its contents").size(20)]
                    .width(Length::Fill)
                    .align_items(Alignment::Center)
                    .spacing(10),
            )
        };

        let content = column![
            combo_box,
            vertical_space().height(50),
            scrollable(selected_file),
        ]
        .width(Length::Fill)
        .align_items(Alignment::Center)
        .spacing(10);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .into()
    }
}

async fn parse(path: PathBuf) -> Arc<HashMap<String, Vec<ConfigPair>>> {
    let data = parse_game(&path);
    Arc::new(data)
}
