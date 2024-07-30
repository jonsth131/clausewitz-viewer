use crate::parser::{ConfigPair, ConfigValue};
use std::{collections::HashMap, fmt, fmt::Display};

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Event {
    pub namespaces: Vec<String>,
    pub country_events: Vec<ConfigValue>,
    pub news_events: Vec<NewsEvent>,
    pub unknown: HashMap<String, ConfigValue>,
}

impl Event {
    pub fn new(pairs: &Vec<ConfigPair>) -> Event {
        let mut event = Event::default();

        for pair in pairs {
            match pair.identifier.as_str() {
                "add_namespace" => event.namespaces.push(pair.value.to_string()),
                "country_event" => event.country_events.push(pair.value.clone()),
                "news_event" => event.news_events.push(NewsEvent::new(&pair.value)),
                _ => {
                    println!(
                        "Unknown event identifier: {}: {}",
                        pair.identifier, pair.value
                    );
                    event
                        .unknown
                        .insert(pair.identifier.clone(), pair.value.clone());
                }
            }
        }

        event
    }
}

impl Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            r#"
Namespaces: {:?}
Country Events: {:?}
Unknown: {:?}"#,
            self.namespaces, self.country_events, self.unknown
        )
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct NewsEvent {
    pub id: String,
    pub title: String,
    pub desc: String,
    pub picture: String,
    pub major: bool,
    pub is_triggered_only: bool,
    pub option: Vec<NewsEventOption>,
    pub trigger: Option<ConfigValue>,
    pub ifs: Vec<ConfigValue>,
}

impl NewsEvent {
    pub fn new(config_value: &ConfigValue) -> NewsEvent {
        let mut news_event = NewsEvent::default();

        let obj = match config_value {
            ConfigValue::Object(obj) => obj,
            _ => {
                println!("Invalid config value for NewsEvent: {:?}", config_value);
                return news_event;
            }
        };

        for pair in obj {
            match pair.identifier.as_str() {
                "id" => news_event.id = pair.value.to_string(),
                "title" => news_event.title = pair.value.to_string(),
                "desc" => news_event.desc = pair.value.to_string(),
                "picture" => news_event.picture = pair.value.to_string(),
                "major" => news_event.major = pair.value.to_bool(),
                "is_triggered_only" => news_event.is_triggered_only = pair.value.to_bool(),
                "option" => {
                    news_event.option.push(NewsEventOption::new(&pair.value));
                }
                "trigger" => news_event.trigger = Some(pair.value.clone()),
                "ifs" => news_event.ifs.push(pair.value.clone()),
                _ => {
                    println!(
                        "Unknown NewsEvent identifier: {}: {}",
                        pair.identifier, pair.value
                    );
                }
            }
        }

        news_event
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct NewsEventOption {
    pub name: String,
    pub desc: String,
    pub log: String,
    pub hidden_effect: Option<ConfigValue>,
    pub trigger: Option<ConfigValue>,
}

impl NewsEventOption {
    pub fn new(config_value: &ConfigValue) -> NewsEventOption {
        let mut option = NewsEventOption::default();

        let obj = match config_value {
            ConfigValue::Object(obj) => obj,
            _ => {
                println!(
                    "Invalid config value for NewsEventOption: {:?}",
                    config_value
                );
                return option;
            }
        };

        for pair in obj {
            match pair.identifier.as_str() {
                "name" => option.name = pair.value.to_string(),
                "desc" => option.desc = pair.value.to_string(),
                "log" => option.log = pair.value.to_string(),
                "hidden_effect" => option.hidden_effect = Some(pair.value.clone()),
                "trigger" => option.trigger = Some(pair.value.clone()),
                _ => {
                    println!(
                        "Unknown NewsEventOption identifier: {}: {}",
                        pair.identifier, pair.value
                    );
                }
            }
        }

        option
    }
}
