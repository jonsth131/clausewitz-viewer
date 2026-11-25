use std::collections::HashMap;

use iced::widget::{
    button, column, container, image, pick_list, row, scrollable, text, text_input, Column,
};
use iced::{Element, Length};

use crate::hoi4::events::{Event, EventFile, EventText};
use crate::parser::ConfigEntry;

#[derive(Debug, Clone)]
pub enum Message {
    SelectFile(String),
    Search(String),
    SelectEvent(usize),
}

#[derive(Debug, Default)]
pub struct EventBrowser {
    files: Vec<EventFile>,
    choices: Vec<String>,
    selected_file: Option<String>,
    selected_event: Option<usize>,
    query: String,
    images: HashMap<String, image::Handle>,
}

impl EventBrowser {
    pub fn new(files: Vec<EventFile>, images: HashMap<String, image::Handle>) -> Self {
        let choices = files.iter().map(|file| file.path.clone()).collect();
        Self {
            files,
            choices,
            images,
            ..Self::default()
        }
    }

    fn file(&self) -> Option<&EventFile> {
        self.files
            .iter()
            .find(|file| Some(&file.path) == self.selected_file.as_ref())
    }

    fn matches(&self, event: &Event) -> bool {
        let query = self.query.to_lowercase();
        event.id.to_lowercase().contains(&query)
            || event.kind.to_lowercase().contains(&query)
            || event
                .titles
                .iter()
                .any(|title| title.text.to_lowercase().contains(&query))
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::SelectFile(path) if self.choices.contains(&path) => {
                if self.selected_file.as_ref() != Some(&path) {
                    self.selected_file = Some(path);
                    self.selected_event = None;
                    self.query.clear();
                }
            }
            Message::Search(query) => {
                self.query = query;
                if self
                    .selected_event
                    .and_then(|index| self.file().and_then(|f| f.events.get(index)))
                    .is_some_and(|event| !self.matches(event))
                {
                    self.selected_event = None;
                }
            }
            Message::SelectEvent(index)
                if self
                    .file()
                    .and_then(|file| file.events.get(index))
                    .is_some_and(|event| self.matches(event)) =>
            {
                self.selected_event = Some(index);
            }
            _ => {}
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let mut content = column![text("Game: Hearts of Iron IV")].spacing(10);
        if self.choices.is_empty() {
            return container(content.push(text("No event files were loaded.")))
                .padding(20)
                .into();
        }
        content = content.push(
            pick_list(
                &self.choices[..],
                self.selected_file.as_ref(),
                Message::SelectFile,
            )
            .placeholder("Choose an event file"),
        );
        if let Some(file) = self.file() {
            let mut list =
                column![text_input("Search event title, ID or type", &self.query)
                    .on_input(Message::Search)]
                .spacing(6);
            let mut entries = Column::new().spacing(4);
            let mut count = 0;
            for (index, event) in file
                .events
                .iter()
                .enumerate()
                .filter(|(_, event)| self.matches(event))
            {
                count += 1;
                let title = event
                    .titles
                    .first()
                    .map_or("<untitled>", |title| title.text.as_str());
                let id = if event.id.is_empty() {
                    "<no ID>"
                } else {
                    &event.id
                };
                entries = entries.push(
                    button(text(format!("{title}\n{} · {id}", event.kind)))
                        .width(Length::Fill)
                        .on_press(Message::SelectEvent(index)),
                );
            }
            if count == 0 {
                entries = entries.push(text(if file.events.is_empty() {
                    "No events defined in this file."
                } else {
                    "No matching events."
                }));
            }
            list = list.push(scrollable(entries).height(Length::Fill));
            let detail = self
                .selected_event
                .and_then(|index| file.events.get(index))
                .map(|event| event_detail(event, &self.images))
                .unwrap_or_else(|| container(text("Select an event.")).padding(10).into());
            content = content.push(
                row![
                    container(list).width(Length::FillPortion(2)),
                    container(detail)
                        .width(Length::FillPortion(3))
                        .height(Length::Fill),
                ]
                .spacing(15)
                .height(Length::Fill),
            );
        }
        container(content).padding(20).height(Length::Fill).into()
    }
}

fn script(entries: &[ConfigEntry]) -> String {
    entries.iter().map(ToString::to_string).collect()
}

fn add_texts<'a>(mut content: Column<'a, Message>, values: &'a [EventText]) -> Column<'a, Message> {
    for value in values {
        content = content.push(text(&value.text));
        if let Some(condition) = &value.condition {
            content = content.push(text(format!("Condition: {condition}")).size(12));
        }
    }
    content
}

fn event_detail<'a>(
    event: &'a Event,
    images: &HashMap<String, image::Handle>,
) -> Element<'a, Message> {
    let mut detail = column![text(format!(
        "{} · {}",
        event.kind,
        if event.id.is_empty() {
            "<no ID>"
        } else {
            &event.id
        }
    ))
    .size(14)]
    .spacing(8);
    if event.titles.is_empty() {
        detail = detail.push(text("<untitled>").size(24));
    } else {
        detail = add_texts(detail, &event.titles);
    }
    if let Some(handle) = event.picture.as_ref().and_then(|name| images.get(name)) {
        detail = detail.push(
            image(handle.clone())
                .width(320)
                .height(180)
                .content_fit(iced::ContentFit::Contain),
        );
    }
    detail = add_texts(detail, &event.descriptions);
    if !event.options.is_empty() {
        detail = detail.push(text("Options").size(20));
        for (index, option) in event.options.iter().enumerate() {
            let mut card = Column::new().spacing(5);
            if option.names.is_empty() {
                card = card.push(text(format!("Option {}", index + 1)));
            } else {
                card = add_texts(card, &option.names);
            }
            card = card.push(text(script(&option.script)).size(12));
            detail = detail.push(
                container(card)
                    .padding(8)
                    .width(Length::Fill)
                    .style(container::rounded_box),
            );
        }
    }
    detail = detail
        .push(text("Event script").size(20))
        .push(text(script(&event.script)).size(12));
    scrollable(detail)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hoi4::events, parser::parse_config_entries};
    use std::collections::BTreeMap;

    #[test]
    fn file_selection_search_and_duplicate_ids_use_source_indices() {
        let config = parse_config_entries(
            "country_event = { id = a title = first } news_event = { id = a title = second }",
        )
        .unwrap();
        let files = events::extract(&BTreeMap::from([
            ("events/a.txt".into(), config),
            ("events/empty.txt".into(), vec![]),
        ]));
        let mut browser = EventBrowser::new(files, HashMap::new());
        browser.update(Message::SelectFile("invalid".into()));
        assert!(browser.selected_file.is_none());
        browser.update(Message::SelectFile("events/a.txt".into()));
        browser.update(Message::Search("SECOND".into()));
        assert!(!browser.matches(&browser.file().unwrap().events[0]));
        assert!(browser.matches(&browser.file().unwrap().events[1]));
        browser.update(Message::SelectEvent(0));
        assert!(browser.selected_event.is_none());
        browser.update(Message::SelectEvent(1));
        assert_eq!(browser.selected_event, Some(1));
        let _ = browser.view();
        browser.update(Message::Search("no match".into()));
        assert!(browser.selected_event.is_none());
        browser.update(Message::SelectFile("events/empty.txt".into()));
        assert!(browser.query.is_empty());
        let _ = browser.view();
    }

    #[test]
    fn renders_selected_event_with_and_without_picture() {
        let config = parse_config_entries(
            "country_event = { id = demo.1 title = label desc = { text = description trigger = { active = yes } } picture = GFX_demo option = { name = choice trigger = { active = yes } } }",
        )
        .unwrap();
        let files = events::extract(&BTreeMap::from([("events/a.txt".into(), config)]));
        let images = HashMap::from([(
            "GFX_demo".into(),
            image::Handle::from_rgba(1, 1, vec![1, 2, 3, 255]),
        )]);
        let mut browser = EventBrowser::new(files, images);
        browser.update(Message::SelectFile("events/a.txt".into()));
        browser.update(Message::SelectEvent(0));
        let _ = browser.view();
        browser.images.clear();
        let _ = browser.view();
    }
}
