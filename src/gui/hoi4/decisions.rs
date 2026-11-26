use crate::hoi4::decisions::Decision;
use crate::parser::ConfigEntry;
use iced::widget::image;
use std::collections::HashMap;

pub use crate::gui::common::script_browser::Message;
use crate::gui::common::script_browser::{Browser, ScriptItem};

pub type DecisionBrowser = Browser<Decision>;

impl ScriptItem for Decision {
    const GAME: &'static str = "Hearts of Iron IV";
    const SINGULAR: &'static str = "decision";
    const PLURAL: &'static str = "decisions";
    const SEARCH: &'static str = "Search decision name, ID or category";

    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn secondary(&self) -> String {
        self.category.clone()
    }
    fn image<'a>(&'a self, _images: &HashMap<String, image::Handle>) -> Option<&'a str> {
        self.icon.as_deref()
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
