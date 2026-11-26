use std::collections::HashMap;

use iced::widget::image;
use iced::{Element, Length};

use crate::common::icons::{IconImage, Icons};
use crate::gui::common::script_browser::{Browser, Message as BrowserMessage, ScriptItem};
use crate::parser::ConfigEntry;
use crate::stellaris::definitions::{DefinitionFile, StellarisDefinition, ViewKind};

pub type Message = StellarisMessage;

#[derive(Debug, Clone)]
pub enum StellarisMessage {
    SelectKind(ViewKind),
    Browser(BrowserMessage),
}

#[derive(Debug)]
pub struct StellarisBrowser {
    selected: ViewKind,
    browsers: HashMap<ViewKind, Browser<StellarisDefinition>>,
}

impl StellarisBrowser {
    pub fn new(files: Vec<DefinitionFile>, images: Icons) -> Self {
        let mut handles = HashMap::<*const IconImage, image::Handle>::new();
        let images = images
            .into_iter()
            .map(|(name, image)| {
                let handle = handles
                    .entry(std::sync::Arc::as_ptr(&image))
                    .or_insert_with(|| {
                        image::Handle::from_rgba(image.width, image.height, image.pixels.clone())
                    });
                (name, handle.clone())
            })
            .collect::<HashMap<_, _>>();
        let browsers = ViewKind::ALL
            .into_iter()
            .map(|kind| {
                let items = files
                    .iter()
                    .filter(|file| file.kind == kind)
                    .map(|file| (file.path.clone(), file.definitions.clone()))
                    .collect();
                (
                    kind,
                    Browser::with_labels(
                        items,
                        images.clone(),
                        kind.singular(),
                        kind.plural(),
                        kind.search_hint(),
                    ),
                )
            })
            .collect();
        Self {
            selected: ViewKind::Civics,
            browsers,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            StellarisMessage::SelectKind(kind) => self.selected = kind,
            StellarisMessage::Browser(message) => {
                if let Some(browser) = self.browsers.get_mut(&self.selected) {
                    browser.update(message);
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let mut tabs = iced::widget::row![].spacing(8);
        for kind in ViewKind::ALL {
            let mut button = iced::widget::button(kind.label());
            if kind != self.selected {
                button = button.on_press(StellarisMessage::SelectKind(kind));
            }
            tabs = tabs.push(button);
        }
        let browser = self
            .browsers
            .get(&self.selected)
            .expect("all Stellaris browser kinds are initialized");
        let tabs = iced::widget::scrollable(tabs)
            .direction(iced::widget::scrollable::Direction::Horizontal(
                iced::widget::scrollable::Scrollbar::default(),
            ))
            .height(44);
        iced::widget::column![tabs, browser.view().map(StellarisMessage::Browser)]
            .spacing(8)
            .height(Length::Fill)
            .into()
    }
}

impl Default for StellarisBrowser {
    fn default() -> Self {
        Self::new(Vec::new(), crate::common::icons::Icons::new())
    }
}

impl ScriptItem for StellarisDefinition {
    const GAME: &'static str = "Stellaris";
    const SINGULAR: &'static str = "definition";
    const PLURAL: &'static str = "definitions";
    const SEARCH: &'static str = "Search name or ID";

    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn secondary(&self) -> String {
        self.secondary.clone()
    }

    fn image<'a>(&'a self, _images: &HashMap<String, image::Handle>) -> Option<&'a str> {
        self.icon.as_deref().or(self.texture.as_deref())
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn sections(&self) -> Vec<(&'static str, String)> {
        Vec::new()
    }

    fn script(&self) -> &[ConfigEntry] {
        &self.script
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_config_entries;
    use crate::stellaris::definitions;
    use std::collections::BTreeMap;

    #[test]
    fn switches_between_views_and_renders_searchable_detail() {
        let files = BTreeMap::from([(
            "common/governments/civics/sample.txt".to_owned(),
            parse_config_entries("civic = { key = civic_a name = civic_a icon = GFX_civic_a }")
                .unwrap(),
        )]);
        let mut browser = StellarisBrowser::new(definitions::extract(&files), Icons::new());
        browser.update(StellarisMessage::SelectKind(ViewKind::Civics));
        browser.update(StellarisMessage::Browser(BrowserMessage::SelectFile(
            "common/governments/civics/sample.txt".into(),
        )));
        browser.update(StellarisMessage::Browser(BrowserMessage::Search(
            "CIVIC_A".into(),
        )));
        browser.update(StellarisMessage::Browser(BrowserMessage::SelectItem(0)));
        let _ = browser.view();
        browser.update(StellarisMessage::SelectKind(ViewKind::Events));
        let _ = browser.view();
    }
}
