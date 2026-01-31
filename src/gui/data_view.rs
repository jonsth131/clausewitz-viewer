use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use iced::widget::{
    button, column, combo_box, container, horizontal_space, row, scrollable, text, vertical_space,
    Column, Row,
};
use iced::{Alignment, Command, Element, Length};
use uuid::Uuid;

use crate::game::parse_game;
use crate::parser::{ConfigPair, ConfigValue};

#[derive(Debug, Clone)]
pub enum Message {
    Selected(String),
    Loaded(Arc<HashMap<String, Vec<ConfigPair>>>),
    Collapse(String),
    Expand(String),
    CollapseAll,
    ExpandAll,
}

#[derive(Debug)]
pub struct DataView {
    is_loading: bool,
    data: HashMap<String, Vec<ConfigPair>>,
    current_open_file: HashMap<String, Vec<DataValue>>,
    files: combo_box::State<String>,
    selected_file: Option<String>,
}

#[derive(Debug, Clone)]
struct DataValue {
    id: String,
    identifier: String,
    sign: String,
    value: String,
    open: bool,
    children: Vec<DataValue>,
}

impl DataView {
    pub fn new(path: PathBuf) -> (Self, iced::Command<Message>) {
        (
            DataView {
                is_loading: true,
                data: HashMap::new(),
                current_open_file: HashMap::new(),
                files: combo_box::State::new(vec![]),
                selected_file: None,
            },
            Command::perform(parse(path.clone()), Message::Loaded),
        )
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        fn traverse(value: &mut DataValue, item: &str, act: fn(&mut DataValue)) {
            if value.id == item {
                act(value);
            } else {
                for child in value.children.iter_mut() {
                    traverse(child, item, act);
                }
            }
        }

        match message {
            Message::Loaded(data) => {
                self.is_loading = false;
                self.data = (*data).clone();
                self.files = combo_box::State::new(self.data.keys().cloned().collect());

                Command::none()
            }
            Message::Selected(file) => {
                self.selected_file = Some(file.clone());

                let data = self.data.get(&file).unwrap();
                self.current_open_file = HashMap::new();

                for pair in data {
                    let value = map_values(pair);

                    if !self.current_open_file.contains_key(&pair.identifier) {
                        self.current_open_file
                            .insert(pair.identifier.clone(), vec![value]);
                    } else {
                        self.current_open_file
                            .get_mut(&pair.identifier)
                            .unwrap()
                            .push(value);
                    }
                }

                Command::none()
            }
            Message::Collapse(item) => {
                fn collapse(value: &mut DataValue) {
                    value.open = false;

                    for child in value.children.iter_mut() {
                        collapse(child);
                    }
                }

                for value in self.current_open_file.values_mut() {
                    for val in value {
                        traverse(val, &item, collapse);
                    }
                }

                Command::none()
            }
            Message::Expand(item) => {
                fn expand(value: &mut DataValue) {
                    value.open = true;
                }

                for value in self.current_open_file.values_mut() {
                    for val in value {
                        traverse(val, &item, expand);
                    }
                }

                Command::none()
            }
            Message::CollapseAll => {
                fn collapse(value: &mut DataValue) {
                    value.open = false;

                    for child in value.children.iter_mut() {
                        collapse(child);
                    }
                }

                for value in self.current_open_file.values_mut() {
                    for val in value {
                        collapse(val);
                    }
                }

                Command::none()
            }
            Message::ExpandAll => {
                fn expand(value: &mut DataValue) {
                    value.open = true;

                    for child in value.children.iter_mut() {
                        expand(child);
                    }
                }

                for value in self.current_open_file.values_mut() {
                    for val in value {
                        expand(val);
                    }
                }

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

        fn create_row(key: &str, value: &DataValue, depth: usize) -> Column<'static, Message> {
            let mut col = Column::new();
            let mut row = Row::new();
            let button_width = 20;
            let indent_width = 25;

            row = row.push(horizontal_space().width((indent_width * depth) as u16));

            if !value.children.is_empty() && !value.open {
                row = row.push(
                    button("+")
                        .width(button_width)
                        .on_press(Message::Expand(value.id.clone())),
                );
            } else if !value.children.is_empty() && value.open {
                row = row.push(
                    button("-")
                        .width(button_width)
                        .on_press(Message::Collapse(value.id.clone())),
                );
            } else {
                row = row.push(horizontal_space().width(button_width));
            }

            row = row.push(horizontal_space().width(10));
            row = row.push(text(format!("{} {} {}", key, value.sign, value.value)));

            col = col.push(row);
            col = col.push(vertical_space().height(10));

            if value.open {
                for child in value.children.iter() {
                    col = col.push(create_row(&child.identifier, child, depth + 1));
                }
            }

            col
        }

        let selected_file = if let Some(_file) = &self.selected_file {
            let mut content = Column::new();
            for (key, value) in self.current_open_file.iter() {
                for val in value.iter() {
                    content = content.push(create_row(key, val, 0));
                }
            }

            container(
                column![
                    row![
                        button("Expand all").on_press(Message::ExpandAll),
                        button("Collapse all").on_press(Message::CollapseAll),
                    ]
                    .spacing(10),
                    content
                ]
                .spacing(10)
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

fn map_values(pair: &ConfigPair) -> DataValue {
    match pair.value {
        ConfigValue::Object(ref children) => DataValue {
            id: Uuid::new_v4().to_string(),
            identifier: pair.identifier.clone(),
            sign: pair.sign.clone(),
            value: "...".to_string(),
            open: false,
            children: children.iter().map(map_values).collect(),
        },
        _ => DataValue {
            id: Uuid::new_v4().to_string(),
            identifier: pair.identifier.clone(),
            sign: pair.sign.clone(),
            value: pair.value.to_string(),
            open: false,
            children: vec![],
        },
    }
}

async fn parse(path: PathBuf) -> Arc<HashMap<String, Vec<ConfigPair>>> {
    let data = parse_game(&path);
    Arc::new(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_values_simple_number_and_string() {
        let pair_num = ConfigPair {
            identifier: "num".to_string(),
            sign: "=".to_string(),
            value: ConfigValue::Number(1.0),
        };

        let dv = map_values(&pair_num);
        assert_eq!(dv.identifier, "num");
        assert_eq!(dv.sign, "=");
        assert_eq!(dv.value, "1");
        assert!(dv.children.is_empty());

        let pair_str = ConfigPair {
            identifier: "s".to_string(),
            sign: "=".to_string(),
            value: ConfigValue::String("hello".to_string()),
        };

        let dv2 = map_values(&pair_str);
        assert_eq!(dv2.identifier, "s");
        assert_eq!(dv2.value, "\"hello\"");
    }

    #[test]
    fn test_map_values_object_nesting() {
        let child = ConfigPair {
            identifier: "child".to_string(),
            sign: "=".to_string(),
            value: ConfigValue::String("v".to_string()),
        };

        let parent = ConfigPair {
            identifier: "parent".to_string(),
            sign: "=".to_string(),
            value: ConfigValue::Object(vec![child.clone()]),
        };

        let dv = map_values(&parent);
        assert_eq!(dv.identifier, "parent");
        assert_eq!(dv.value, "...");
        assert_eq!(dv.children.len(), 1);
        assert_eq!(dv.children[0].identifier, "child");
        assert_eq!(dv.children[0].value, "\"v\"");
    }

    #[test]
    fn test_dataview_selected_groups() {
        let mut view = DataView {
            is_loading: false,
            data: HashMap::new(),
            current_open_file: HashMap::new(),
            files: combo_box::State::new(vec![]),
            selected_file: None,
        };

        let pair1 = ConfigPair {
            identifier: "grp".to_string(),
            sign: "=".to_string(),
            value: ConfigValue::Number(2.0),
        };

        // two entries with same identifier should group under the same key
        let entries = vec![pair1.clone(), pair1.clone()];
        let mut map = HashMap::new();
        map.insert("file1".to_string(), entries);

        let arc = Arc::new(map);
        // simulate Loaded
        let _ = view.update(Message::Loaded(arc.clone()));
        assert!(!view.is_loading);
        assert!(view.data.contains_key("file1"));

        // select file1
        let _ = view.update(Message::Selected("file1".to_string()));
        assert_eq!(view.selected_file.as_ref().map(String::as_str), Some("file1"));
        assert!(view.current_open_file.contains_key("grp"));
        assert_eq!(view.current_open_file.get("grp").unwrap().len(), 2);
    }
}
