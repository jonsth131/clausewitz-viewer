use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use crate::parser::{ConfigEntry, ConfigPair, ConfigValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewKind {
    Civics,
    Origins,
    Traditions,
    AscensionPerks,
    Technologies,
    PoliciesAndEdicts,
    Species,
    Events,
}

impl ViewKind {
    pub const ALL: [Self; 8] = [
        Self::Civics,
        Self::Origins,
        Self::Traditions,
        Self::AscensionPerks,
        Self::Technologies,
        Self::PoliciesAndEdicts,
        Self::Species,
        Self::Events,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Civics => "Civics",
            Self::Origins => "Origins",
            Self::Traditions => "Traditions",
            Self::AscensionPerks => "Ascension Perks",
            Self::Technologies => "Technologies",
            Self::PoliciesAndEdicts => "Policies & Edicts",
            Self::Species => "Species",
            Self::Events => "Events",
        }
    }

    pub fn singular(self) -> &'static str {
        match self {
            Self::Civics => "civic",
            Self::Origins => "origin",
            Self::Traditions => "tradition",
            Self::AscensionPerks => "ascension perk",
            Self::Technologies => "technology",
            Self::PoliciesAndEdicts => "policy or edict",
            Self::Species => "species",
            Self::Events => "event",
        }
    }

    pub fn plural(self) -> &'static str {
        match self {
            Self::Civics => "civics",
            Self::Origins => "origins",
            Self::Traditions => "traditions",
            Self::AscensionPerks => "ascension perks",
            Self::Technologies => "technologies",
            Self::PoliciesAndEdicts => "policies and edicts",
            Self::Species => "species",
            Self::Events => "events",
        }
    }

    pub fn search_hint(self) -> &'static str {
        match self {
            Self::Civics => "Search civic name or ID",
            Self::Origins => "Search origin name or ID",
            Self::Traditions => "Search tradition name or ID",
            Self::AscensionPerks => "Search ascension perk name or ID",
            Self::Technologies => "Search technology name or ID",
            Self::PoliciesAndEdicts => "Search policy or edict name or ID",
            Self::Species => "Search species name or ID",
            Self::Events => "Search event name or ID",
        }
    }

    fn path_matches(self, path: &str) -> bool {
        match self {
            Self::Civics => {
                path.starts_with("common/governments/civics/") && !path.contains("origin")
            }
            Self::Origins => {
                path.starts_with("common/governments/civics/") && path.contains("origin")
            }
            Self::Traditions => path.starts_with("common/traditions/"),
            Self::AscensionPerks => path.starts_with("common/ascension_perks/"),
            Self::Technologies => path.starts_with("common/technology/"),
            Self::PoliciesAndEdicts => {
                path.starts_with("common/policies/") || path.starts_with("common/edicts/")
            }
            Self::Species => {
                path.starts_with("common/species_classes/") || path.starts_with("common/species/")
            }
            Self::Events => path.starts_with("events/"),
        }
    }

    fn wrappers(self) -> &'static [&'static str] {
        match self {
            Self::Civics => &["civic", "civics"],
            Self::Origins => &["origin", "origins"],
            Self::Traditions => &["tradition", "traditions", "tradition_category"],
            Self::AscensionPerks => &["ascension_perk", "ascension_perks"],
            Self::Technologies => &["technology", "technologies"],
            Self::PoliciesAndEdicts => &["policy", "policies", "edict", "edicts"],
            Self::Species => &["species", "species_class"],
            Self::Events => &[
                "country_event",
                "fleet_event",
                "planet_event",
                "ship_event",
                "observation_station_event",
                "megastructure_event",
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub struct StellarisDefinition {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub secondary: String,
    pub icon: Option<String>,
    pub texture: Option<String>,
    pub script: Vec<ConfigEntry>,
    name_key: String,
    description_key: String,
}

#[derive(Debug, Clone)]
pub struct DefinitionFile {
    pub path: String,
    pub kind: ViewKind,
    pub definitions: Vec<StellarisDefinition>,
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

fn field<'a>(fields: &'a [ConfigEntry], key: &str) -> Option<&'a str> {
    pairs(fields)
        .find(|pair| pair.identifier == key)
        .and_then(|pair| scalar(&pair.value))
        .filter(|value| !value.is_empty())
}

fn image_reference(fields: &[ConfigEntry]) -> (Option<String>, Option<String>) {
    let Some(value) = pairs(fields)
        .filter(|pair| matches!(pair.identifier.as_str(), "icon" | "picture" | "portrait"))
        .filter_map(|pair| scalar(&pair.value))
        .find(|value| !value.contains(['[', '$']))
    else {
        return (None, None);
    };
    if value.starts_with("GFX_") {
        (Some(value.to_owned()), None)
    } else if value.contains('/')
        || Path::new(value)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("dds"))
    {
        (None, Some(value.to_owned()))
    } else {
        (Some(format!("GFX_{value}")), None)
    }
}

fn make_definition(kind: ViewKind, id: &str, fields: &[ConfigEntry]) -> StellarisDefinition {
    let name_key = field(fields, "name")
        .or_else(|| {
            (kind == ViewKind::Events)
                .then(|| field(fields, "title"))
                .flatten()
        })
        .unwrap_or(id)
        .to_owned();
    let description_key = field(fields, "desc")
        .or_else(|| field(fields, "description"))
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{id}_desc"));
    let category = field(fields, "category").or_else(|| field(fields, "area"));
    let secondary = match (category, field(fields, "tier")) {
        (Some(category), Some(tier)) => format!("{category} · tier {tier}"),
        (Some(category), None) => category.to_owned(),
        (None, Some(tier)) => format!("Tier {tier}"),
        (None, None) => kind.label().to_owned(),
    };
    let (icon, texture) = image_reference(fields);
    StellarisDefinition {
        id: id.to_owned(),
        name: name_key.clone(),
        description: None,
        secondary,
        icon,
        texture,
        script: fields.to_vec(),
        name_key,
        description_key,
    }
}

pub fn extract(files: &BTreeMap<String, Vec<ConfigEntry>>) -> Vec<DefinitionFile> {
    let mut result = Vec::new();
    for (path, entries) in files {
        for kind in ViewKind::ALL {
            if !kind.path_matches(path) {
                continue;
            }
            let mut definitions = Vec::new();
            for entry in pairs(entries) {
                let ConfigValue::Object(fields) = &entry.value else {
                    continue;
                };
                if kind == ViewKind::Events && entry.identifier.ends_with("_event") {
                    let id = field(fields, "id").unwrap_or(&entry.identifier);
                    definitions.push(make_definition(kind, id, fields));
                } else if kind.wrappers().contains(&entry.identifier.as_str()) {
                    if matches!(
                        entry.identifier.as_str(),
                        "civic"
                            | "origin"
                            | "tradition"
                            | "ascension_perk"
                            | "technology"
                            | "policy"
                            | "edict"
                            | "species"
                            | "species_class"
                    ) && ["id", "key", "name", "icon", "archetype", "portrait"]
                        .iter()
                        .any(|marker| field(fields, marker).is_some())
                    {
                        let id = field(fields, "key")
                            .or_else(|| field(fields, "id"))
                            .unwrap_or(&entry.identifier);
                        definitions.push(make_definition(kind, id, fields));
                    } else {
                        for child in pairs(fields) {
                            if let ConfigValue::Object(definition) = &child.value {
                                definitions.push(make_definition(
                                    kind,
                                    &child.identifier,
                                    definition,
                                ));
                            }
                        }
                    }
                } else {
                    definitions.push(make_definition(kind, &entry.identifier, fields));
                }
            }
            if !definitions.is_empty() || kind.path_matches(path) {
                result.push(DefinitionFile {
                    path: path.clone(),
                    kind,
                    definitions,
                });
            }
        }
    }
    result
}

pub fn localization_keys(files: &[DefinitionFile]) -> HashSet<String> {
    files
        .iter()
        .flat_map(|file| &file.definitions)
        .flat_map(|definition| {
            [
                definition.name_key.clone(),
                definition.description_key.clone(),
            ]
        })
        .collect()
}

pub fn apply_localization(files: &mut [DefinitionFile], values: &HashMap<String, String>) {
    for definition in files.iter_mut().flat_map(|file| &mut file.definitions) {
        if let Some(name) = values
            .get(&definition.name_key)
            .filter(|value| !value.is_empty())
        {
            definition.name.clone_from(name);
        }
        definition.description = values
            .get(&definition.description_key)
            .filter(|value| !value.is_empty())
            .cloned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_config_entries;

    #[test]
    fn extracts_and_localizes_definitions_without_losing_duplicate_ids() {
        let files = BTreeMap::from([(
            "common/ascension_perks/test.txt".into(),
            parse_config_entries(
                "ap = { name = perk_one icon = GFX_perk_one desc = perk_desc } \
                 ap = { name = perk_two icon = GFX_perk_two }",
            )
            .unwrap(),
        )]);
        let mut extracted = extract(&files);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].kind, ViewKind::AscensionPerks);
        assert_eq!(extracted[0].definitions.len(), 2);
        assert_eq!(
            extracted[0].definitions[0].icon.as_deref(),
            Some("GFX_perk_one")
        );
        assert_eq!(extracted[0].definitions[0].script.len(), 3);
        apply_localization(
            &mut extracted,
            &HashMap::from([
                ("perk_one".to_owned(), "First Perk".to_owned()),
                ("perk_desc".to_owned(), "A description".to_owned()),
            ]),
        );
        assert_eq!(extracted[0].definitions[0].name, "First Perk");
        assert_eq!(
            extracted[0].definitions[0].description.as_deref(),
            Some("A description")
        );
    }

    #[test]
    fn recognizes_event_scopes_and_keeps_raw_options() {
        let files = BTreeMap::from([(
            "events/sample.txt".into(),
            parse_config_entries(
                "planet_event = { id = test.1 title = event_title option = { name = choice } }",
            )
            .unwrap(),
        )]);
        let extracted = extract(&files);
        assert_eq!(extracted[0].kind, ViewKind::Events);
        assert_eq!(extracted[0].definitions[0].id, "test.1");
        assert_eq!(extracted[0].definitions[0].name, "event_title");
        assert_eq!(extracted[0].definitions[0].script.len(), 3);
    }

    #[test]
    fn extracts_the_selected_definition_families_from_synthetic_paths() {
        let files = BTreeMap::from([
            (
                "common/governments/civics/civics.txt".into(),
                parse_config_entries("civic = { key = civic_a name = civic_a icon = GFX_civic_a }")
                    .unwrap(),
            ),
            (
                "common/governments/civics/00_origins.txt".into(),
                parse_config_entries("origin = { key = origin_a name = origin_a }").unwrap(),
            ),
            (
                "common/traditions/traditions.txt".into(),
                parse_config_entries("tradition = { key = tradition_a name = tradition_a }")
                    .unwrap(),
            ),
            (
                "common/ascension_perks/perks.txt".into(),
                parse_config_entries("perk_a = { name = perk_a }").unwrap(),
            ),
            (
                "common/technology/technology.txt".into(),
                parse_config_entries("technology = { key = tech_a name = tech_a icon = tech_a }")
                    .unwrap(),
            ),
            (
                "common/policies/policies.txt".into(),
                parse_config_entries("policy = { key = policy_a name = policy_a }").unwrap(),
            ),
            (
                "common/species_classes/species.txt".into(),
                parse_config_entries("species_class = { key = species_a name = species_a }")
                    .unwrap(),
            ),
            (
                "events/events.txt".into(),
                parse_config_entries("country_event = { id = event_a title = event_a }").unwrap(),
            ),
        ]);
        let extracted = extract(&files);
        assert_eq!(extracted.len(), ViewKind::ALL.len());
        for kind in ViewKind::ALL {
            let file = extracted
                .iter()
                .find(|file| file.kind == kind)
                .expect("each selected view has a synthetic file");
            assert_eq!(file.definitions.len(), 1, "{}", kind.label());
        }
        let technology = extracted
            .iter()
            .find(|file| file.kind == ViewKind::Technologies)
            .unwrap();
        assert_eq!(
            technology.definitions[0].icon.as_deref(),
            Some("GFX_tech_a")
        );
    }
}
