use std::path::PathBuf;
use std::sync::Arc;

use iced::widget::{column, combo_box, container, row, scrollable, text, vertical_space};
use iced::{Alignment, Command, Element, Length};

use crate::hoi4::{parse_hoi4, Hoi4};

#[derive(Debug, Clone)]
pub enum Message {
    Selected(String),
    Loaded(Arc<Hoi4>),
}

#[derive(Debug)]
pub struct Hoi4View {
    hoi4: Hoi4,
    is_loading: bool,
    countries: combo_box::State<String>,
    selected_country: Option<String>,
}

impl Hoi4View {
    pub fn new(path: PathBuf) -> (Self, iced::Command<Message>) {
        (
            Hoi4View {
                hoi4: Hoi4::default(),
                is_loading: true,
                countries: combo_box::State::new(vec![]),
                selected_country: None,
            },
            Command::perform(parse(path.clone()), Message::Loaded),
        )
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Loaded(hoi4) => {
                self.is_loading = false;
                self.hoi4 = (*hoi4).clone();
                self.countries =
                    combo_box::State::new(self.hoi4.countries.keys().cloned().collect());

                Command::none()
            }
            Message::Selected(country) => {
                self.selected_country = Some(country);
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
            &self.countries,
            "Select a country",
            self.selected_country.as_ref(),
            Message::Selected,
        )
        .width(450);

        let selected_country = if let Some(country) = &self.selected_country {
            let country = self.hoi4.countries.get(country).unwrap();
            let first_row = row![
                column![
                    text("Starting values:").size(20),
                    vertical_space().height(10),
                    text(format!("Capital: {}", &country.capital)),
                    text(format!("Research slots: {}", &country.research_slots)),
                    text(format!("Convoys: {}", &country.convoys)),
                    text(format!("Trains: {}", &country.trains)),
                    text(format!("Stability: {}", &country.stability)),
                    text(format!("War support: {}", &country.war_support)),
                    text(format!("Command power: {}", &country.command_power)),
                ],
                column![
                    text("Politics:").size(20),
                    vertical_space().height(10),
                    text(&country.politics),
                ],
                column![
                    text("Popularities:").size(20),
                    vertical_space().height(10),
                    text(&country.popularities),
                ],
            ]
            .spacing(20);

            let second_row = row![
                column![
                    text("Technology:").size(20),
                    vertical_space().height(10),
                    container(
                        scrollable(column![text(country.technology.join("\n"))])
                            .width(250)
                            .height(150)
                    )
                ],
                column![
                    text("Ideas:").size(20),
                    vertical_space().height(10),
                    container(
                        scrollable(column![text(country.ideas.join("\n"))])
                            .width(250)
                            .height(150)
                    )
                ],
                column![
                    text("Stockpile:").size(20),
                    vertical_space().height(10),
                    container(
                        scrollable(column![text(
                            country
                                .stockpile
                                .clone()
                                .into_iter()
                                .map(|i| i.to_string())
                                .collect::<Vec<String>>()
                                .join("\n")
                        )])
                        .width(250)
                        .height(150)
                    )
                ],
            ]
            .spacing(20);

            let third_row = row![
                column![
                    text("Recruit Characters:").size(20),
                    vertical_space().height(10),
                    container(
                        scrollable(column![text(country.recruit_characters.join("\n"))])
                            .width(250)
                            .height(150)
                    )
                ],
                column![
                    text("Created Country Leaders:").size(20),
                    vertical_space().height(10),
                    container(
                        scrollable(column![text(
                            country
                                .created_country_leaders
                                .clone()
                                .into_iter()
                                .map(|i| format!("{}", i))
                                .collect::<Vec<String>>()
                                .join("\n\n")
                        )])
                        .width(500)
                        .height(150)
                    )
                ],
            ]
            .spacing(20);

            let start_1939 = if country.start_1939.is_empty() {
                column![]
            } else {
                column![
                    text("1939 Start:").size(20),
                    vertical_space().height(10),
                    container(
                        scrollable(column![text(
                            country
                                .start_1939
                                .clone()
                                .into_iter()
                                .map(|v| format!("{}", v))
                                .collect::<Vec<String>>()
                                .join("\n")
                        )])
                        .width(750)
                        .height(150)
                    )
                ]
            };

            let variables = if country.variables.is_empty() {
                column![]
            } else {
                column![
                    text("Variables:").size(20),
                    vertical_space().height(10),
                    container(
                        scrollable(column![text(
                            country
                                .variables
                                .clone()
                                .into_iter()
                                .map(|v| format!("{}", v))
                                .collect::<Vec<String>>()
                                .join("\n")
                        )])
                        .width(750)
                        .height(150)
                    )
                ]
            };

            let ifs = if country.ifs.is_empty() {
                column![]
            } else {
                column![
                    text("Ifs:").size(20),
                    vertical_space().height(10),
                    container(
                        scrollable(column![text(
                            country
                                .ifs
                                .clone()
                                .into_iter()
                                .map(|v| format!("{}", v))
                                .collect::<Vec<String>>()
                                .join("\n")
                        )])
                        .width(750)
                        .height(150)
                    )
                ]
            };

            let unknown_fields = if country.unknown.is_empty() {
                column![]
            } else {
                column![
                    text("Unknown fields:").size(20),
                    vertical_space().height(10),
                    container(
                        scrollable(column![text(
                            country
                                .unknown
                                .keys()
                                .cloned()
                                .collect::<Vec<String>>()
                                .join("\n")
                        )])
                        .width(750)
                        .height(250)
                    )
                ]
            };

            column![
                first_row,
                second_row,
                third_row,
                start_1939,
                ifs,
                variables,
                unknown_fields
            ]
            .spacing(20)
        } else {
            column![text("No country selected")]
        };

        let content = column![combo_box, vertical_space().height(50), selected_country]
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

async fn parse(path: PathBuf) -> Arc<Hoi4> {
    let hoi4 = parse_hoi4(&path);
    Arc::new(hoi4)
}
