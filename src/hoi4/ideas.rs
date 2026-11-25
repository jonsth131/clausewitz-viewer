use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use crate::parser::{ConfigEntry, ConfigPair, ConfigValue};

#[derive(Debug, Clone)]
pub struct Idea {
    pub id: String,
    pub family: String,
    pub name: String,
    pub description: Option<String>,
    pub picture: Option<String>,
    pub script: Vec<ConfigEntry>,
    name_key: String,
    description_key: String,
}

#[derive(Debug, Clone)]
pub struct IdeaFile {
    pub path: String,
    pub ideas: Vec<Idea>,
}

fn pairs(entries: &[ConfigEntry]) -> impl Iterator<Item = &ConfigPair> {
    entries.iter().filter_map(|entry| match entry {
        ConfigEntry::Pair(pair) => Some(pair),
        ConfigEntry::Value(_) => None,
    })
}

fn scalar(value: &ConfigValue) -> Option<&str> {
    match value {
        ConfigValue::String(value) | ConfigValue::Identifier(value) => Some(value),
        _ => None,
    }
}

pub fn extract(files: &BTreeMap<String, Vec<ConfigEntry>>) -> Vec<IdeaFile> {
    files
        .iter()
        .filter(|(path, _)| Path::new(path).starts_with("common/ideas"))
        .map(|(path, entries)| {
            let mut ideas = Vec::new();
            for wrapper in pairs(entries).filter(|pair| pair.identifier == "ideas") {
                let ConfigValue::Object(families) = &wrapper.value else {
                    continue;
                };
                for family in pairs(families) {
                    let ConfigValue::Object(children) = &family.value else {
                        continue;
                    };
                    for idea in pairs(children) {
                        let ConfigValue::Object(fields) = &idea.value else {
                            continue;
                        };
                        let field = |key| {
                            pairs(fields)
                                .find(|p| p.identifier == key)
                                .and_then(|p| scalar(&p.value))
                                .filter(|s| !s.is_empty())
                        };
                        let name_key = field("name").unwrap_or(&idea.identifier).to_owned();
                        let description_key = field("desc")
                            .map(str::to_owned)
                            .unwrap_or_else(|| format!("{}_desc", idea.identifier));
                        let picture = field("picture").and_then(|value| {
                            if value.contains('[') || value.contains('$') {
                                None
                            } else if value.starts_with("GFX_") {
                                Some(value.to_owned())
                            } else {
                                Some(format!("GFX_idea_{value}"))
                            }
                        });
                        ideas.push(Idea {
                            id: idea.identifier.clone(),
                            family: family.identifier.clone(),
                            name: idea.identifier.clone(),
                            description: None,
                            picture,
                            script: fields.clone(),
                            name_key,
                            description_key,
                        });
                    }
                }
            }
            IdeaFile {
                path: path.clone(),
                ideas,
            }
        })
        .collect()
}

pub(super) fn localization_keys(files: &[IdeaFile]) -> HashSet<String> {
    files
        .iter()
        .flat_map(|file| &file.ideas)
        .flat_map(|idea| [idea.name_key.clone(), idea.description_key.clone()])
        .collect()
}

pub(super) fn apply_localization(files: &mut [IdeaFile], values: &HashMap<String, String>) {
    for idea in files.iter_mut().flat_map(|file| &mut file.ideas) {
        if let Some(name) = values.get(&idea.name_key).filter(|value| !value.is_empty()) {
            idea.name.clone_from(name);
        }
        idea.description = values
            .get(&idea.description_key)
            .filter(|value| !value.is_empty())
            .cloned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_config_entries;

    #[test]
    fn extracts_families_duplicates_and_preserves_script_order() {
        let config = parse_config_entries(
            r#"
          ideas = {
            country = {
              alpha = { picture = example modifier = { stability = 1 } modifier = { gain = 2 } }
              alpha = { name = override desc = custom_desc picture = GFX_direct }
            }
            advisors = {
              beta = { picture = [GetPicture] allowed = { enabled = yes } }
              delta = { picture = GFX_[GetPicture] }
            }
          }
          ideas = { industry = { gamma = { } } }
        "#,
        )
        .unwrap();
        let mut files = extract(&BTreeMap::from([
            ("common/ideas/sample.txt".into(), config.clone()),
            ("common/ideas_extra/sample.txt".into(), config),
        ]));
        assert_eq!(files.len(), 1);
        let ideas = &files[0].ideas;
        assert_eq!(
            ideas
                .iter()
                .map(|idea| (&*idea.id, &*idea.family))
                .collect::<Vec<_>>(),
            [
                ("alpha", "country"),
                ("alpha", "country"),
                ("beta", "advisors"),
                ("delta", "advisors"),
                ("gamma", "industry")
            ]
        );
        assert_eq!(ideas[0].picture.as_deref(), Some("GFX_idea_example"));
        assert_eq!(ideas[1].picture.as_deref(), Some("GFX_direct"));
        assert!(ideas[2].picture.is_none());
        assert!(ideas[3].picture.is_none());
        assert_eq!(
            ideas[0]
                .script
                .iter()
                .filter(|entry| entry.to_string().starts_with("modifier"))
                .count(),
            2
        );
        apply_localization(
            &mut files,
            &HashMap::from([
                ("alpha".into(), "First".into()),
                ("alpha_desc".into(), "Description".into()),
                ("override".into(), "Second".into()),
                ("custom_desc".into(), "Custom".into()),
            ]),
        );
        let ideas = &files[0].ideas;
        assert_eq!(ideas[0].name, "First");
        assert_eq!(ideas[0].description.as_deref(), Some("Description"));
        assert_eq!(ideas[1].name, "Second");
        assert_eq!(ideas[1].description.as_deref(), Some("Custom"));
        assert_eq!(ideas[2].name, "beta");
        assert!(ideas[2].description.is_none());
    }
}
