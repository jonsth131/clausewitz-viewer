pub use crate::gui::common::script_browser::Message;
use crate::gui::common::script_browser::{Browser, ScriptItem};
use crate::hoi4::technologies::Technology;
use crate::parser::ConfigEntry;
use iced::widget::image;
use std::collections::HashMap;

pub type TechnologyBrowser = Browser<Technology>;

impl ScriptItem for Technology {
    const GAME: &'static str = "Hearts of Iron IV";
    const SINGULAR: &'static str = "technology";
    const PLURAL: &'static str = "technologies";
    const SEARCH: &'static str = "Search technology name or ID";

    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn secondary(&self) -> String {
        "Technology".into()
    }
    fn image<'a>(&'a self, _images: &HashMap<String, image::Handle>) -> Option<&'a str> {
        self.icon.as_deref()
    }
    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
    fn sections(&self) -> Vec<(&'static str, String)> {
        vec![
            ("Paths and prerequisites", self.relationships.concat()),
            ("Linked from", self.linked_from.join(", ")),
            ("Unlocks", self.unlocks.concat()),
        ]
    }
    fn script(&self) -> &[ConfigEntry] {
        &self.script
    }
}
