use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use super::script::{pairs, scalar, static_sprite};
use crate::parser::{ConfigEntry, ConfigValue};

#[derive(Debug, Clone)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub roles: Vec<String>,
    pub portraits: Vec<String>,
    pub portrait_groups: Vec<PortraitGroup>,
    pub role_script: Vec<String>,
    pub script: Vec<ConfigEntry>,
    name_key: String,
    description_key: String,
}

#[derive(Debug, Clone)]
pub struct PortraitGroup {
    pub category: String,
    pub candidates: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CharacterFile {
    pub path: String,
    pub characters: Vec<Character>,
}

fn collect_portraits<'a>(entries: &'a [ConfigEntry], portraits: &mut [Vec<&'a str>; 3]) {
    for pair in pairs(entries) {
        if let Some(name) = scalar(&pair.value).and_then(static_sprite) {
            let priority = match pair.identifier.as_str() {
                "large" => 0,
                "small" => 1,
                _ => 2,
            };
            portraits[priority].push(name);
        } else if let ConfigValue::Object(children) = &pair.value {
            collect_portraits(children, portraits);
        }
    }
}

pub fn extract(files: &BTreeMap<String, Vec<ConfigEntry>>) -> Vec<CharacterFile> {
    files
        .iter()
        .filter(|(path, _)| Path::new(path).starts_with("common/characters"))
        .map(|(path, entries)| {
            let mut characters = Vec::new();
            for wrapper in pairs(entries).filter(|pair| pair.identifier == "characters") {
                let ConfigValue::Object(items) = &wrapper.value else {
                    continue;
                };
                for item in pairs(items) {
                    let ConfigValue::Object(fields) = &item.value else {
                        continue;
                    };
                    // Fields may also live in DLC-conditional `instance` blocks.
                    let scopes: Vec<&[ConfigEntry]> = std::iter::once(fields.as_slice())
                        .chain(
                            pairs(fields)
                                .filter(|p| p.identifier == "instance")
                                .filter_map(|p| match &p.value {
                                    ConfigValue::Object(children) => Some(children.as_slice()),
                                    _ => None,
                                }),
                        )
                        .collect();
                    let field = |key| {
                        scopes.iter().find_map(|scope| {
                            pairs(scope)
                                .find(|p| p.identifier == key)
                                .and_then(|p| scalar(&p.value))
                        })
                    };
                    let name_key = field("name").unwrap_or(&item.identifier).to_owned();
                    let description_key = field("desc")
                        .map(str::to_owned)
                        .unwrap_or_else(|| format!("{}_desc", item.identifier));
                    let mut candidates = [Vec::new(), Vec::new(), Vec::new()];
                    let mut grouped: Vec<(String, [Vec<&str>; 3])> = Vec::new();
                    for portrait in scopes
                        .iter()
                        .flat_map(|scope| pairs(scope))
                        .filter(|pair| pair.identifier == "portraits")
                    {
                        if let ConfigValue::Object(children) = &portrait.value {
                            collect_portraits(children, &mut candidates);
                            for category in pairs(children) {
                                if let ConfigValue::Object(entries) = &category.value {
                                    let index = grouped
                                        .iter()
                                        .position(|(name, _)| *name == category.identifier)
                                        .unwrap_or_else(|| {
                                            grouped.push((
                                                category.identifier.clone(),
                                                Default::default(),
                                            ));
                                            grouped.len() - 1
                                        });
                                    collect_portraits(entries, &mut grouped[index].1);
                                }
                            }
                        }
                    }
                    let portrait_groups = grouped
                        .into_iter()
                        .map(|(category, group)| {
                            let mut names = Vec::<String>::new();
                            for name in group.into_iter().flatten() {
                                if !names.iter().any(|existing| existing == name) {
                                    names.push(name.to_owned());
                                }
                            }
                            PortraitGroup {
                                category,
                                candidates: names,
                            }
                        })
                        .collect();
                    let mut portraits = Vec::new();
                    for name in candidates.into_iter().flatten() {
                        if !portraits.iter().any(|existing| existing == name) {
                            portraits.push(name.to_owned());
                        }
                    }
                    let mut roles: Vec<&crate::parser::ConfigPair> = Vec::new();
                    for role in scopes.iter().flat_map(|scope| pairs(scope)).filter(|pair| {
                        matches!(
                            pair.identifier.as_str(),
                            "advisor"
                                | "country_leader"
                                | "corps_commander"
                                | "navy_leader"
                                | "operative"
                                | "unit_leader"
                                | "operative_leader"
                                | "scientist"
                        )
                    }) {
                        if !roles
                            .iter()
                            .any(|existing| existing.to_string() == role.to_string())
                        {
                            roles.push(role);
                        }
                    }
                    for role in &roles {
                        if let ConfigValue::Object(role_fields) = &role.value {
                            for image in pairs(role_fields)
                                .filter(|field| {
                                    matches!(field.identifier.as_str(), "picture" | "icon")
                                })
                                .filter_map(|field| scalar(&field.value).and_then(static_sprite))
                            {
                                if !portraits.iter().any(|existing| existing == image) {
                                    portraits.push(image.to_owned());
                                }
                            }
                        }
                    }
                    characters.push(Character {
                        id: item.identifier.clone(),
                        name: name_key.clone(),
                        description: None,
                        roles: roles.iter().fold(Vec::new(), |mut names, role| {
                            if !names.contains(&role.identifier) {
                                names.push(role.identifier.clone());
                            }
                            names
                        }),
                        portraits,
                        portrait_groups,
                        role_script: roles.iter().map(ToString::to_string).collect(),
                        script: fields.clone(),
                        name_key,
                        description_key,
                    });
                }
            }
            CharacterFile {
                path: path.clone(),
                characters,
            }
        })
        .collect()
}

pub(super) fn localization_keys(files: &[CharacterFile]) -> HashSet<String> {
    files
        .iter()
        .flat_map(|file| &file.characters)
        .flat_map(|character| {
            [
                character.name_key.clone(),
                character.description_key.clone(),
            ]
        })
        .collect()
}

pub(super) fn apply_localization(files: &mut [CharacterFile], values: &HashMap<String, String>) {
    for character in files.iter_mut().flat_map(|file| &mut file.characters) {
        if let Some(name) = values
            .get(&character.name_key)
            .filter(|value| !value.is_empty())
        {
            character.name.clone_from(name);
        }
        character.description = values
            .get(&character.description_key)
            .filter(|value| !value.is_empty())
            .cloned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_config_entries;

    #[test]
    fn reads_portraits_roles_and_names_from_instance_blocks() {
        let entries = parse_config_entries(
            r#"
          characters = {
            person = {
              instance = { allowed = { has_dlc = "X" } name = label
                portraits = { civilian = { large = GFX_big small = GFX_small } }
                advisor = { slot = political_advisor } }
              instance = { portraits = { civilian = { small = GFX_generic } }
                advisor = { slot = political_advisor } }
            }
          }
        "#,
        )
        .unwrap();
        let files = extract(&BTreeMap::from([(
            "common/characters/a.txt".into(),
            entries,
        )]));
        let person = &files[0].characters[0];
        assert_eq!(person.name, "label");
        assert_eq!(person.portraits, ["GFX_big", "GFX_small", "GFX_generic"]);
        assert_eq!(person.portrait_groups.len(), 1);
        assert_eq!(
            person.portrait_groups[0].candidates,
            ["GFX_big", "GFX_small", "GFX_generic"]
        );
        assert_eq!(person.roles, ["advisor"]);
        assert_eq!(person.role_script.len(), 1);
    }

    #[test]
    fn extracts_duplicate_characters_roles_portraits_and_localization() {
        let entries = parse_config_entries(r#"
          characters = {
            person = { name = label portraits = { civilian = { large = GFX_person small = GFX_small } }
                       advisor = { slot = political_advisor traits = { skilled } picture = GFX_advisor } }
            person = { name = missing portraits = { civilian = { large = GFX_[Dynamic] small = "$ICON$" } }
                       country_leader = { traits = { leader_trait } } }
          }
        "#).unwrap();
        let mut files = extract(&BTreeMap::from([
            ("common/characters/a.txt".into(), entries.clone()),
            ("common/characters_extra/a.txt".into(), entries),
        ]));
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].characters.len(), 2);
        assert_eq!(
            files[0].characters[0].portraits,
            ["GFX_person", "GFX_small", "GFX_advisor"]
        );
        assert_eq!(files[0].characters[0].roles, ["advisor"]);
        assert!(files[0].characters[0].role_script[0].contains("skilled"));
        assert!(files[0].characters[1].portraits.is_empty());
        apply_localization(
            &mut files,
            &HashMap::from([("label".into(), "A name".into())]),
        );
        assert_eq!(files[0].characters[0].name, "A name");
        assert_eq!(files[0].characters[1].name, "missing");
    }

    #[test]
    fn prioritizes_large_portraits_across_scopes_and_keeps_fallbacks() {
        let entries = parse_config_entries(
            r#"characters = {
                first = {
                    portraits = {
                        civilian = { small = GFX_small other = GFX_other large = GFX_large }
                        army = { small = GFX_army_small large = GFX_army_large }
                    }
                    advisor = { picture = GFX_advisor }
                }
                second = { portraits = { civilian = { small = GFX_only } } }
                third = { portraits = { civilian = { small = GFX_fallback large = GFX_[Dynamic] } } }
            }"#,
        )
        .unwrap();
        let files = extract(&BTreeMap::from([(
            "common/characters/a.txt".into(),
            entries,
        )]));
        let characters = &files[0].characters;
        assert_eq!(
            characters[0].portraits,
            [
                "GFX_large",
                "GFX_army_large",
                "GFX_small",
                "GFX_army_small",
                "GFX_other",
                "GFX_advisor"
            ]
        );
        assert_eq!(characters[1].portraits, ["GFX_only"]);
        assert_eq!(characters[2].portraits, ["GFX_fallback"]);
        assert_eq!(characters[0].portrait_groups[0].category, "civilian");
        assert_eq!(
            characters[0].portrait_groups[0].candidates,
            ["GFX_large", "GFX_small", "GFX_other"]
        );
        assert_eq!(characters[0].portrait_groups[1].category, "army");
        assert_eq!(
            characters[0].portrait_groups[1].candidates,
            ["GFX_army_large", "GFX_army_small"]
        );
    }
}
