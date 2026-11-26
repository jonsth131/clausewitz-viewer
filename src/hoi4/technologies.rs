use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use super::script::{pairs, scalar, static_sprite};
use crate::parser::{ConfigEntry, ConfigValue};

#[derive(Debug, Clone)]
pub struct Technology {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub implicit_icon: bool,
    pub relationships: Vec<String>,
    pub linked_from: Vec<String>,
    pub unlocks: Vec<String>,
    pub script: Vec<ConfigEntry>,
}

#[derive(Debug, Clone)]
pub struct TechnologyFile {
    pub path: String,
    pub technologies: Vec<Technology>,
}

pub fn extract(files: &BTreeMap<String, Vec<ConfigEntry>>) -> Vec<TechnologyFile> {
    let mut result: Vec<_> = files
        .iter()
        .filter(|(path, _)| Path::new(path).starts_with("common/technologies"))
        .map(|(path, entries)| {
            let mut technologies = Vec::new();
            for wrapper in pairs(entries).filter(|pair| pair.identifier == "technologies") {
                let ConfigValue::Object(items) = &wrapper.value else {
                    continue;
                };
                for item in pairs(items) {
                    let ConfigValue::Object(fields) = &item.value else {
                        continue;
                    };
                    let explicit_icon = pairs(fields)
                        .find(|field| field.identifier == "icon")
                        .and_then(|field| scalar(&field.value))
                        .and_then(static_sprite);
                    let implicit_icon = explicit_icon.is_none();
                    let icon = explicit_icon.map(str::to_owned).or_else(|| {
                        (!item.identifier.contains(['[', '$']))
                            .then(|| format!("GFX_{}", item.identifier))
                    });
                    let relationships = pairs(fields)
                        .filter(|field| {
                            matches!(
                                field.identifier.as_str(),
                                "path" | "prerequisites" | "prerequisite" | "leads_to_tech"
                            )
                        })
                        .map(ToString::to_string)
                        .collect();
                    let unlocks = pairs(fields)
                        .filter(|field| {
                            field.identifier.starts_with("enable_")
                                || field.identifier.starts_with("unlock_")
                        })
                        .map(ToString::to_string)
                        .collect();
                    technologies.push(Technology {
                        id: item.identifier.clone(),
                        name: item.identifier.clone(),
                        description: None,
                        icon,
                        implicit_icon,
                        relationships,
                        linked_from: Vec::new(),
                        unlocks,
                        script: fields.clone(),
                    });
                }
            }
            TechnologyFile {
                path: path.clone(),
                technologies,
            }
        })
        .collect();
    let mut incoming = HashMap::<String, Vec<String>>::new();
    for tech in result.iter().flat_map(|file| &file.technologies) {
        for path in pairs(&tech.script).filter(|pair| pair.identifier == "path") {
            if let ConfigValue::Object(fields) = &path.value {
                for target in pairs(fields)
                    .filter(|pair| pair.identifier == "leads_to_tech")
                    .filter_map(|pair| scalar(&pair.value))
                {
                    incoming
                        .entry(target.to_owned())
                        .or_default()
                        .push(tech.id.clone());
                }
            }
        }
    }
    for tech in result.iter_mut().flat_map(|file| &mut file.technologies) {
        if let Some(sources) = incoming.get(&tech.id) {
            tech.linked_from.clone_from(sources);
        }
    }
    result
}

pub(super) fn localization_keys(files: &[TechnologyFile]) -> HashSet<String> {
    files
        .iter()
        .flat_map(|file| &file.technologies)
        .flat_map(|tech| [tech.id.clone(), format!("{}_desc", tech.id)])
        .collect()
}

pub(super) fn apply_localization(files: &mut [TechnologyFile], values: &HashMap<String, String>) {
    for tech in files.iter_mut().flat_map(|file| &mut file.technologies) {
        if let Some(name) = values.get(&tech.id).filter(|value| !value.is_empty()) {
            tech.name.clone_from(name);
        }
        tech.description = values
            .get(&format!("{}_desc", tech.id))
            .filter(|value| !value.is_empty())
            .cloned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_config_entries;

    #[test]
    fn extracts_order_and_cross_file_links_without_evaluating_conditions() {
        let files = BTreeMap::from([
            ("common/technologies/a.txt".into(), parse_config_entries(
                "technologies = { alpha = { path = { leads_to_tech = beta condition = { flag = yes } } enable_equipment = { item_a } } alpha = { icon = GFX_custom } }"
            ).unwrap()),
            ("common/technologies/sub/b.txt".into(), parse_config_entries(
                "technologies = { beta = { prerequisites = { alpha } } }"
            ).unwrap()),
            ("common/technologies_extra/a.txt".into(), vec![]),
        ]);
        let mut files = extract(&files);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].technologies.len(), 2);
        assert_eq!(files[0].technologies[0].icon.as_deref(), Some("GFX_alpha"));
        assert!(files[0].technologies[0].implicit_icon);
        assert_eq!(files[0].technologies[1].icon.as_deref(), Some("GFX_custom"));
        assert!(!files[0].technologies[1].implicit_icon);
        assert!(files[0].technologies[0].relationships[0].contains("condition"));
        assert!(files[0].technologies[0].unlocks[0].contains("item_a"));
        assert_eq!(files[1].technologies[0].linked_from, ["alpha"]);
        apply_localization(
            &mut files,
            &HashMap::from([
                ("alpha".into(), "Localized".into()),
                ("alpha_desc".into(), "Details".into()),
            ]),
        );
        assert_eq!(files[0].technologies[0].name, "Localized");
        assert_eq!(
            files[0].technologies[0].description.as_deref(),
            Some("Details")
        );
        assert_eq!(files[1].technologies[0].name, "beta");
    }
}
