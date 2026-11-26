use std::collections::HashMap;

use iced::widget::{
    button, column, container, image, pick_list, row, scrollable, text, text_input, Column,
};
use iced::{Alignment, Element, Length};

use crate::parser::ConfigEntry;

pub trait ScriptItem {
    const GAME: &'static str;
    const SINGULAR: &'static str;
    const PLURAL: &'static str;
    const SEARCH: &'static str;

    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn secondary(&self) -> String;
    fn image<'a>(&'a self, images: &HashMap<String, image::Handle>) -> Option<&'a str>;
    fn detail_images<'a>(
        &'a self,
        images: &HashMap<String, image::Handle>,
    ) -> Vec<(&'a str, &'a str)> {
        self.image(images)
            .map(|name| vec![("", name)])
            .unwrap_or_default()
    }
    fn description(&self) -> Option<&str>;
    fn sections(&self) -> Vec<(&'static str, String)>;
    fn script(&self) -> &[ConfigEntry];
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectFile(String),
    Search(String),
    SelectItem(usize),
}

#[derive(Debug)]
pub struct Browser<T> {
    files: Vec<(String, Vec<T>)>,
    choices: Vec<String>,
    selected_file: Option<String>,
    selected_item: Option<usize>,
    query: String,
    images: HashMap<String, image::Handle>,
    singular: &'static str,
    plural: &'static str,
    search_hint: &'static str,
}

impl<T> Default for Browser<T> {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            choices: Vec::new(),
            selected_file: None,
            selected_item: None,
            query: String::new(),
            images: HashMap::new(),
            singular: "",
            plural: "",
            search_hint: "",
        }
    }
}

impl<T: ScriptItem> Browser<T> {
    pub fn new(files: Vec<(String, Vec<T>)>, images: HashMap<String, image::Handle>) -> Self {
        Self::with_labels(files, images, T::SINGULAR, T::PLURAL, T::SEARCH)
    }

    pub fn with_labels(
        files: Vec<(String, Vec<T>)>,
        images: HashMap<String, image::Handle>,
        singular: &'static str,
        plural: &'static str,
        search_hint: &'static str,
    ) -> Self {
        let choices = files.iter().map(|(path, _)| path.clone()).collect();
        Self {
            files,
            choices,
            images,
            singular,
            plural,
            search_hint,
            ..Self::default()
        }
    }

    fn file(&self) -> Option<&[T]> {
        self.files
            .iter()
            .find(|(path, _)| Some(path) == self.selected_file.as_ref())
            .map(|(_, items)| items.as_slice())
    }

    fn matches(&self, item: &T) -> bool {
        let query = self.query.to_lowercase();
        item.name().to_lowercase().contains(&query)
            || item.id().to_lowercase().contains(&query)
            || item.secondary().to_lowercase().contains(&query)
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::SelectFile(path) if self.choices.contains(&path) => {
                if self.selected_file.as_ref() != Some(&path) {
                    self.selected_file = Some(path);
                    self.selected_item = None;
                    self.query.clear();
                }
            }
            Message::Search(query) => {
                self.query = query;
                if self
                    .selected_item
                    .and_then(|index| self.file().and_then(|items| items.get(index)))
                    .is_some_and(|item| !self.matches(item))
                {
                    self.selected_item = None;
                }
            }
            Message::SelectItem(index)
                if self
                    .file()
                    .and_then(|items| items.get(index))
                    .is_some_and(|item| self.matches(item)) =>
            {
                self.selected_item = Some(index)
            }
            _ => {}
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let mut content = column![text(format!("Game: {}", T::GAME))].spacing(10);
        if self.choices.is_empty() {
            return container(content.push(text(format!("No {} files were loaded.", self.plural))))
                .padding(20)
                .into();
        }
        content = content.push(
            pick_list(
                &self.choices[..],
                self.selected_file.as_ref(),
                Message::SelectFile,
            )
            .placeholder(format!("Choose a {} file", T::SINGULAR)),
        );
        if let Some(items) = self.file() {
            let mut list =
                column![text_input(self.search_hint, &self.query).on_input(Message::Search)]
                    .spacing(6);
            let mut entries = Column::new().spacing(4);
            let mut count = 0;
            for (index, item) in items
                .iter()
                .enumerate()
                .filter(|(_, item)| self.matches(item))
            {
                count += 1;
                entries = entries.push(
                    button(item_row(item, &self.images))
                        .width(Length::Fill)
                        .on_press(Message::SelectItem(index)),
                );
            }
            if count == 0 {
                entries = entries.push(text(if items.is_empty() {
                    format!("No {} defined in this file.", self.plural)
                } else {
                    format!("No matching {}.", self.plural)
                }));
            }
            list = list.push(scrollable(entries).height(Length::Fill));
            let detail = self
                .selected_item
                .and_then(|index| items.get(index))
                .map(|item| detail(item, &self.images, self.singular))
                .unwrap_or_else(|| {
                    container(text(format!("Select a {}.", self.singular)))
                        .padding(10)
                        .into()
                });
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

fn item_row<'a, T: ScriptItem>(
    item: &'a T,
    images: &HashMap<String, image::Handle>,
) -> Element<'a, Message> {
    let mut label = row![].spacing(8).align_y(Alignment::Center);
    if let Some(handle) = item.image(images).and_then(|name| images.get(name)) {
        label = label.push(
            image(handle.clone())
                .width(48)
                .height(48)
                .content_fit(iced::ContentFit::Contain),
        );
    }
    label
        .push(text(format!(
            "{}\n{} · {}",
            item.name(),
            item.secondary(),
            item.id()
        )))
        .into()
}

fn detail<'a, T: ScriptItem>(
    item: &'a T,
    images: &HashMap<String, image::Handle>,
    singular: &str,
) -> Element<'a, Message> {
    let mut content = column![
        text(item.name()).size(24),
        text(format!("{} · {}", item.secondary(), item.id())).size(14)
    ]
    .spacing(8);
    for (label, name) in item.detail_images(images) {
        let Some(handle) = images.get(name) else {
            continue;
        };
        if !label.is_empty() {
            content = content.push(text(label));
        }
        content = content.push(
            image(handle.clone())
                .width(120)
                .height(120)
                .content_fit(iced::ContentFit::Contain),
        );
    }
    if let Some(description) = item.description() {
        content = content.push(text(description));
    }
    for (heading, value) in item.sections() {
        if !value.is_empty() {
            content = content.push(text(heading).size(20)).push(text(value));
        }
    }
    let script: String = item.script().iter().map(ToString::to_string).collect();
    content = content
        .push(text(format!("{singular} script")).size(20))
        .push(text(script).size(12));
    scrollable(content)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        hoi4::{characters, decisions, technologies},
        parser::parse_config_entries,
    };
    use std::collections::BTreeMap;

    #[test]
    fn searches_and_selects_duplicate_ids_by_source_index() {
        let entries =
            parse_config_entries("first = { a = { icon = test } } second = { a = { cost = 1 } }")
                .unwrap();
        let files = decisions::extract(&BTreeMap::from([
            ("common/decisions/a.txt".into(), entries),
            ("common/decisions/b.txt".into(), vec![]),
        ]));
        let mut browser = Browser::new(
            files
                .into_iter()
                .map(|file| (file.path, file.decisions))
                .collect(),
            HashMap::new(),
        );
        browser.update(Message::SelectFile("invalid".into()));
        assert!(browser.selected_file.is_none());
        browser.update(Message::SelectFile("common/decisions/a.txt".into()));
        browser.update(Message::Search("SECOND".into()));
        browser.update(Message::SelectItem(0));
        assert!(browser.selected_item.is_none());
        browser.update(Message::SelectItem(1));
        assert_eq!(browser.selected_item, Some(1));
        let _ = browser.view();
        browser.update(Message::Search("missing".into()));
        assert!(browser.selected_item.is_none());
        browser.update(Message::SelectFile("common/decisions/b.txt".into()));
        assert!(browser.query.is_empty());
        assert!(browser.file().unwrap().is_empty());
        let _ = browser.view();
    }

    #[test]
    fn technology_and_character_selection_render_with_image_fallbacks() {
        let tech = technologies::extract(&BTreeMap::from([(
            "common/technologies/a.txt".into(),
            parse_config_entries("technologies = { alpha = { path = { leads_to_tech = beta } } }")
                .unwrap(),
        )]));
        let mut tech_browser = Browser::new(
            tech.into_iter()
                .map(|file| (file.path, file.technologies))
                .collect(),
            HashMap::from([(
                "GFX_alpha".into(),
                image::Handle::from_rgba(1, 1, vec![1, 2, 3, 255]),
            )]),
        );
        tech_browser.update(Message::SelectFile("common/technologies/a.txt".into()));
        tech_browser.update(Message::Search("ALPHA".into()));
        tech_browser.update(Message::SelectItem(0));
        assert_eq!(tech_browser.selected_item, Some(0));
        let _ = tech_browser.view();

        let characters = characters::extract(&BTreeMap::from([(
            "common/characters/a.txt".into(),
            parse_config_entries("characters = { person = { portraits = { civilian = { small = GFX_present large = GFX_large } } advisor = { traits = { skilled } } } }").unwrap(),
        )]));
        let mut character_browser = Browser::new(
            characters
                .into_iter()
                .map(|file| (file.path, file.characters))
                .collect(),
            HashMap::from([
                (
                    "GFX_present".into(),
                    image::Handle::from_rgba(1, 1, vec![1, 2, 3, 255]),
                ),
                (
                    "GFX_large".into(),
                    image::Handle::from_rgba(1, 1, vec![4, 5, 6, 255]),
                ),
            ]),
        );
        character_browser.update(Message::SelectFile("common/characters/a.txt".into()));
        character_browser.update(Message::Search("ADVISOR".into()));
        character_browser.update(Message::SelectItem(0));
        let character = &character_browser.file().unwrap()[0];
        assert_eq!(
            character.image(&character_browser.images),
            Some("GFX_large")
        );
        let _ = character_browser.view();
        character_browser.images.remove("GFX_large");
        let character = &character_browser.file().unwrap()[0];
        assert_eq!(
            character.image(&character_browser.images),
            Some("GFX_present")
        );
        let _ = character_browser.view();
        character_browser.images.clear();
        assert!(character_browser.file().unwrap()[0]
            .image(&character_browser.images)
            .is_none());
        let _ = character_browser.view();
    }

    #[test]
    fn decision_icon_remains_to_the_left_of_the_title() {
        use iced::advanced::graphics::core::renderer::Headless;
        use iced::advanced::{layout, widget::Tree, Layout};
        use iced::{Renderer, Size};
        let files = decisions::extract(&BTreeMap::from([(
            "common/decisions/a.txt".into(),
            parse_config_entries("group = { alpha = { icon = test } }").unwrap(),
        )]));
        let images = HashMap::from([(
            "GFX_decision_test".into(),
            image::Handle::from_rgba(1, 1, vec![1, 2, 3, 255]),
        )]);
        let mut row = item_row(&files[0].decisions[0], &images);
        let renderer = iced::futures::executor::block_on(<Renderer as Headless>::new(
            iced::Font::default(),
            16.into(),
            Some("tiny-skia"),
        ))
        .expect("software renderer");
        let mut tree = Tree::new(row.as_widget());
        let node = row.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(500.0, 100.0)),
        );
        let layout = Layout::new(&node);
        let mut children = layout.children();
        let icon = children.next().unwrap().bounds();
        let title = children.next().unwrap().bounds();
        assert_eq!(icon.width, 48.0);
        assert!(icon.x + icon.width <= title.x);
    }

    #[test]
    fn displays_each_portrait_category_with_per_category_fallback() {
        let files = characters::extract(&BTreeMap::from([(
            "common/characters/a.txt".into(),
            parse_config_entries("characters = { person = { portraits = { civilian = { small = GFX_civil_small large = GFX_civil_large } army = { small = GFX_army_small large = GFX_missing } navy = { large = GFX_[Dynamic] small = GFX_navy_small } } } }").unwrap(),
        )]));
        let character = &files[0].characters[0];
        let images = [
            "GFX_civil_large",
            "GFX_civil_small",
            "GFX_army_small",
            "GFX_navy_small",
        ]
        .into_iter()
        .map(|name| {
            (
                name.to_owned(),
                image::Handle::from_rgba(1, 1, vec![1, 2, 3, 255]),
            )
        })
        .collect();
        assert_eq!(
            character.detail_images(&images),
            [
                ("civilian", "GFX_civil_large"),
                ("army", "GFX_army_small"),
                ("navy", "GFX_navy_small"),
            ]
        );
        let _ = detail(character, &images, "character");
    }
}
