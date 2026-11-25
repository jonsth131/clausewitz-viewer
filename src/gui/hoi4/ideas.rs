use std::collections::HashMap;

use iced::widget::{
    button, column, container, image, pick_list, row, scrollable, text, text_input, Column,
};
use iced::{Alignment, Element, Length};

use crate::hoi4::ideas::{Idea, IdeaFile};

#[derive(Debug, Clone)]
pub enum Message {
    SelectFile(String),
    Search(String),
    SelectIdea(usize),
}

#[derive(Debug, Default)]
pub struct IdeaBrowser {
    files: Vec<IdeaFile>,
    choices: Vec<String>,
    selected_file: Option<String>,
    selected_idea: Option<usize>,
    query: String,
    images: HashMap<String, image::Handle>,
}

impl IdeaBrowser {
    pub fn new(files: Vec<IdeaFile>, images: HashMap<String, image::Handle>) -> Self {
        let choices = files.iter().map(|file| file.path.clone()).collect();
        Self {
            files,
            choices,
            images,
            ..Self::default()
        }
    }

    fn file(&self) -> Option<&IdeaFile> {
        self.files
            .iter()
            .find(|file| Some(&file.path) == self.selected_file.as_ref())
    }

    fn matches(&self, idea: &Idea) -> bool {
        let query = self.query.to_lowercase();
        idea.name.to_lowercase().contains(&query)
            || idea.id.to_lowercase().contains(&query)
            || idea.family.to_lowercase().contains(&query)
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::SelectFile(path) if self.choices.contains(&path) => {
                if self.selected_file.as_ref() != Some(&path) {
                    self.selected_file = Some(path);
                    self.selected_idea = None;
                    self.query.clear();
                }
            }
            Message::Search(query) => {
                self.query = query;
                if self
                    .selected_idea
                    .and_then(|index| self.file().and_then(|file| file.ideas.get(index)))
                    .is_some_and(|idea| !self.matches(idea))
                {
                    self.selected_idea = None;
                }
            }
            Message::SelectIdea(index)
                if self
                    .file()
                    .and_then(|file| file.ideas.get(index))
                    .is_some_and(|idea| self.matches(idea)) =>
            {
                self.selected_idea = Some(index)
            }
            _ => {}
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let mut content = column![text("Game: Hearts of Iron IV")].spacing(10);
        if self.choices.is_empty() {
            return container(content.push(text("No idea files were loaded.")))
                .padding(20)
                .into();
        }
        content = content.push(
            pick_list(
                &self.choices[..],
                self.selected_file.as_ref(),
                Message::SelectFile,
            )
            .placeholder("Choose an idea file"),
        );
        if let Some(file) = self.file() {
            let mut list =
                column![text_input("Search idea name, ID or family", &self.query)
                    .on_input(Message::Search)]
                .spacing(6);
            let mut entries = Column::new().spacing(4);
            let mut count = 0;
            for (index, idea) in file
                .ideas
                .iter()
                .enumerate()
                .filter(|(_, idea)| self.matches(idea))
            {
                count += 1;
                let mut label = row![].spacing(8).align_y(Alignment::Center);
                if let Some(handle) = idea
                    .picture
                    .as_ref()
                    .and_then(|picture| self.images.get(picture))
                {
                    label = label.push(
                        image(handle.clone())
                            .width(48)
                            .height(48)
                            .content_fit(iced::ContentFit::Contain),
                    );
                }
                label = label.push(text(format!(
                    "{}\n{} · {}",
                    idea.name, idea.family, idea.id
                )));
                entries = entries.push(
                    button(label)
                        .width(Length::Fill)
                        .on_press(Message::SelectIdea(index)),
                );
            }
            if count == 0 {
                entries = entries.push(text(if file.ideas.is_empty() {
                    "No ideas defined in this file."
                } else {
                    "No matching ideas."
                }));
            }
            list = list.push(scrollable(entries).height(Length::Fill));
            let detail = self
                .selected_idea
                .and_then(|index| file.ideas.get(index))
                .map(|idea| idea_detail(idea, &self.images))
                .unwrap_or_else(|| container(text("Select an idea.")).padding(10).into());
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

fn idea_detail<'a>(
    idea: &'a Idea,
    images: &HashMap<String, image::Handle>,
) -> Element<'a, Message> {
    let mut content = column![
        text(&idea.name).size(24),
        text(format!("{} · {}", idea.family, idea.id)).size(14),
    ]
    .spacing(8);
    if let Some(handle) = idea
        .picture
        .as_ref()
        .and_then(|picture| images.get(picture))
    {
        content = content.push(
            image(handle.clone())
                .width(96)
                .height(96)
                .content_fit(iced::ContentFit::Contain),
        );
    }
    if let Some(description) = &idea.description {
        content = content.push(text(description));
    }
    let script: String = idea.script.iter().map(ToString::to_string).collect();
    content = content
        .push(text("Idea script").size(20))
        .push(text(script).size(12));
    scrollable(content)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hoi4::ideas, parser::parse_config_entries};
    use std::collections::BTreeMap;

    #[test]
    fn selects_and_filters_by_family_and_duplicate_ids() {
        let config = parse_config_entries(
            "ideas = { country = { alpha = { picture = example } } advisor = { alpha = { } } }",
        )
        .unwrap();
        let files = ideas::extract(&BTreeMap::from([
            ("common/ideas/a.txt".into(), config),
            ("common/ideas/empty.txt".into(), vec![]),
        ]));
        let mut browser = IdeaBrowser::new(files, HashMap::new());
        browser.update(Message::SelectFile("invalid".into()));
        assert!(browser.selected_file.is_none());
        browser.update(Message::SelectFile("common/ideas/a.txt".into()));
        browser.update(Message::Search("ADVISOR".into()));
        assert!(!browser.matches(&browser.file().unwrap().ideas[0]));
        assert!(browser.matches(&browser.file().unwrap().ideas[1]));
        browser.update(Message::SelectIdea(0));
        assert!(browser.selected_idea.is_none());
        browser.update(Message::SelectIdea(1));
        assert_eq!(browser.selected_idea, Some(1));
        let _ = browser.view();
        browser.update(Message::Search("none".into()));
        assert!(browser.selected_idea.is_none());
        browser.update(Message::SelectFile("common/ideas/empty.txt".into()));
        assert!(browser.query.is_empty());
        let _ = browser.view();
    }

    #[test]
    fn renders_selected_idea_with_and_without_picture() {
        let config =
            parse_config_entries("ideas = { country = { alpha = { picture = sample cost = 5 } } }")
                .unwrap();
        let files = ideas::extract(&BTreeMap::from([("common/ideas/a.txt".into(), config)]));
        let images = HashMap::from([(
            "GFX_idea_sample".into(),
            image::Handle::from_rgba(1, 1, vec![1, 2, 3, 255]),
        )]);
        let mut browser = IdeaBrowser::new(files, images);
        browser.update(Message::SelectFile("common/ideas/a.txt".into()));
        browser.update(Message::SelectIdea(0));
        let _ = browser.view();
        browser.images.clear();
        let _ = browser.view();
    }
}
