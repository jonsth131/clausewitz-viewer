pub use crate::gui::common::script_browser::Message;
use crate::gui::common::script_browser::{Browser, ScriptItem};
use crate::hoi4::characters::Character;
use crate::parser::ConfigEntry;
use iced::widget::image;
use std::collections::HashMap;

pub type CharacterBrowser = Browser<Character>;

impl ScriptItem for Character {
    const GAME: &'static str = "Hearts of Iron IV";
    const SINGULAR: &'static str = "character";
    const PLURAL: &'static str = "characters";
    const SEARCH: &'static str = "Search character name, ID or role";

    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn secondary(&self) -> String {
        if self.roles.is_empty() {
            "Character".into()
        } else {
            self.roles.join(", ")
        }
    }
    fn image<'a>(&'a self, images: &HashMap<String, image::Handle>) -> Option<&'a str> {
        self.portraits
            .iter()
            .find(|name| images.contains_key(*name))
            .map(String::as_str)
    }
    fn detail_images<'a>(
        &'a self,
        images: &HashMap<String, image::Handle>,
    ) -> Vec<(&'a str, &'a str)> {
        let groups: Vec<_> = self
            .portrait_groups
            .iter()
            .filter_map(|group| {
                group
                    .candidates
                    .iter()
                    .find(|name| images.contains_key(*name))
                    .map(|name| (group.category.as_str(), name.as_str()))
            })
            .collect();
        if groups.is_empty() {
            self.image(images)
                .map(|name| vec![("", name)])
                .unwrap_or_default()
        } else {
            groups
        }
    }
    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
    fn sections(&self) -> Vec<(&'static str, String)> {
        vec![("Roles and traits", self.role_script.concat())]
    }
    fn script(&self) -> &[ConfigEntry] {
        &self.script
    }
}
