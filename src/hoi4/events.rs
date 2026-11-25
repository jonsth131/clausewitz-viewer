use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use crate::parser::{ConfigEntry, ConfigPair, ConfigValue};

#[derive(Debug, Clone)]
pub struct EventText {
    pub key: String,
    pub text: String,
    pub condition: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EventOption {
    pub names: Vec<EventText>,
    pub script: Vec<ConfigEntry>,
}

#[derive(Debug, Clone)]
pub struct Event {
    pub kind: String,
    pub id: String,
    pub titles: Vec<EventText>,
    pub descriptions: Vec<EventText>,
    pub picture: Option<String>,
    pub options: Vec<EventOption>,
    pub script: Vec<ConfigEntry>,
}

#[derive(Debug, Clone)]
pub struct EventFile {
    pub path: String,
    pub events: Vec<Event>,
}

fn scalar(value: &ConfigValue) -> Option<String> {
    match value {
        ConfigValue::Identifier(s) | ConfigValue::String(s) => Some(s.clone()),
        _ => None,
    }
}

fn pairs(entries: &[ConfigEntry]) -> impl Iterator<Item = &ConfigPair> {
    entries.iter().filter_map(|entry| match entry {
        ConfigEntry::Pair(pair) => Some(pair),
        ConfigEntry::Value(_) => None,
    })
}

fn localized_text(value: &ConfigValue) -> Option<EventText> {
    let (key, condition) = match value {
        ConfigValue::Object(entries) => {
            let key = pairs(entries)
                .find(|p| p.identifier == "text")
                .and_then(|p| scalar(&p.value))?;
            let condition = pairs(entries)
                .find(|p| p.identifier == "trigger")
                .map(ToString::to_string);
            (key, condition)
        }
        _ => (scalar(value)?, None),
    };
    Some(EventText {
        text: key.clone(),
        key,
        condition,
    })
}

pub fn extract(files: &BTreeMap<String, Vec<ConfigEntry>>) -> Vec<EventFile> {
    files
        .iter()
        .filter(|(path, _)| Path::new(path).starts_with("events"))
        .map(|(path, entries)| {
            let events = pairs(entries)
                .filter(|pair| {
                    matches!(
                        pair.identifier.as_str(),
                        "country_event"
                            | "news_event"
                            | "state_event"
                            | "unit_leader_event"
                            | "operative_leader_event"
                    )
                })
                .filter_map(|pair| {
                    let ConfigValue::Object(fields) = &pair.value else {
                        return None;
                    };
                    let id = pairs(fields)
                        .find(|p| p.identifier == "id")
                        .and_then(|p| scalar(&p.value))
                        .unwrap_or_default();
                    let titles = pairs(fields)
                        .filter(|p| p.identifier == "title")
                        .filter_map(|p| localized_text(&p.value))
                        .collect();
                    let descriptions = pairs(fields)
                        .filter(|p| p.identifier == "desc")
                        .filter_map(|p| localized_text(&p.value))
                        .collect();
                    let picture = pairs(fields)
                        .filter(|p| p.identifier == "picture")
                        .filter_map(|p| scalar(&p.value))
                        .find(|value| value.starts_with("GFX_"));
                    let options = pairs(fields)
                        .filter(|p| p.identifier == "option")
                        .filter_map(|p| {
                            let ConfigValue::Object(fields) = &p.value else {
                                return None;
                            };
                            Some(EventOption {
                                names: pairs(fields)
                                    .filter(|p| p.identifier == "name")
                                    .filter_map(|p| localized_text(&p.value))
                                    .collect(),
                                script: fields.clone(),
                            })
                        })
                        .collect();
                    Some(Event {
                        kind: pair.identifier.clone(),
                        id,
                        titles,
                        descriptions,
                        picture,
                        options,
                        script: fields.clone(),
                    })
                })
                .collect();
            EventFile {
                path: path.clone(),
                events,
            }
        })
        .collect()
}

fn texts(event: &Event) -> impl Iterator<Item = &EventText> {
    event
        .titles
        .iter()
        .chain(&event.descriptions)
        .chain(event.options.iter().flat_map(|option| &option.names))
}

pub(super) fn localization_keys(files: &[EventFile]) -> HashSet<String> {
    files
        .iter()
        .flat_map(|file| &file.events)
        .flat_map(texts)
        .map(|value| value.key.clone())
        .collect()
}

pub(super) fn apply_localization(files: &mut [EventFile], values: &HashMap<String, String>) {
    for event in files.iter_mut().flat_map(|file| &mut file.events) {
        let texts = event
            .titles
            .iter_mut()
            .chain(&mut event.descriptions)
            .chain(
                event
                    .options
                    .iter_mut()
                    .flat_map(|option| &mut option.names),
            );
        for value in texts {
            if let Some(text) = values.get(&value.key).filter(|s| !s.is_empty()) {
                value.text.clone_from(text);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_config_entries;

    #[test]
    fn extracts_top_level_events_and_preserves_conditional_text_and_nested_calls() {
        let input = r#"
          add_namespace = sample
          country_event = {
            id = sample.1 title = title_key
            desc = { text = desc_one trigger = { flag = yes } }
            desc = desc_two picture = GFX_sample
            option = { name = choice_key trigger = { yes = yes } country_event = { id = nested.1 } }
            option = { name = other_key effect = { flag = no } }
          }
          news_event = { id = sample.2 title = absent picture = [GetPicture] }
        "#;
        let files = BTreeMap::from([
            ("events/a.txt".into(), parse_config_entries(input).unwrap()),
            ("other/a.txt".into(), parse_config_entries(input).unwrap()),
        ]);
        let mut files = extract(&files);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].events.len(), 2);
        let event = &files[0].events[0];
        assert_eq!(event.id, "sample.1");
        assert_eq!(event.descriptions.len(), 2);
        assert!(event.descriptions[0]
            .condition
            .as_ref()
            .unwrap()
            .contains("flag"));
        assert_eq!(event.picture.as_deref(), Some("GFX_sample"));
        assert_eq!(event.options.len(), 2);
        assert!(event.options[0]
            .script
            .iter()
            .any(|entry| entry.to_string().contains("nested.1")));
        assert_eq!(files[0].events[1].picture, None);
        apply_localization(
            &mut files,
            &HashMap::from([
                ("title_key".into(), "Title".into()),
                ("choice_key".into(), "Choice".into()),
            ]),
        );
        assert_eq!(files[0].events[0].titles[0].text, "Title");
        assert_eq!(files[0].events[0].options[0].names[0].text, "Choice");
        assert_eq!(files[0].events[0].descriptions[0].text, "desc_one");
    }
}
