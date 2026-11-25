use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use crate::parser::{ConfigEntry, ConfigValue};

use super::concise;

#[derive(Debug, Clone)]
pub struct Decision {
    pub id: String,
    pub category: String,
    pub name: String,
    pub tooltip: String,
    pub icon: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DecisionFile {
    pub path: String,
    pub decisions: Vec<Decision>,
}

pub fn extract(files: &BTreeMap<String, Vec<ConfigEntry>>) -> Vec<DecisionFile> {
    let root = Path::new("common/decisions");
    let categories = root.join("categories");
    let mut result = Vec::new();
    for (path, entries) in files {
        let path_ref = Path::new(path);
        if !path_ref.starts_with(root) || path_ref.starts_with(&categories) {
            continue;
        }
        let mut decisions = Vec::new();
        for category in entries {
            let ConfigEntry::Pair(category) = category else {
                continue;
            };
            let ConfigValue::Object(entries) = &category.value else {
                continue;
            };
            for entry in entries {
                let ConfigEntry::Pair(pair) = entry else {
                    continue;
                };
                let ConfigValue::Object(fields) = &pair.value else {
                    continue;
                };
                let mut tooltip =
                    format!("ID: {}\nCategory: {}", pair.identifier, category.identifier);
                let mut icon = None;
                for field in fields {
                    if let ConfigEntry::Pair(field) = field {
                        if field.identifier == "icon" && icon.is_none() {
                            let value = match &field.value {
                                ConfigValue::Identifier(value) | ConfigValue::String(value) => {
                                    Some(value)
                                }
                                _ => None,
                            };
                            icon = value.filter(|value| !value.is_empty()).map(|value| {
                                if value.starts_with("GFX_") {
                                    value.clone()
                                } else {
                                    format!("GFX_decision_{value}")
                                }
                            });
                        }
                    }
                    let text = field
                        .to_string()
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    tooltip.push_str(&format!("\n{}", concise(&text, 400)));
                }
                decisions.push(Decision {
                    id: pair.identifier.clone(),
                    category: category.identifier.clone(),
                    name: pair.identifier.clone(),
                    tooltip: concise(&tooltip, 4000),
                    icon,
                });
            }
        }
        result.push(DecisionFile {
            path: path.clone(),
            decisions,
        });
    }
    result
}

pub(super) fn localization_keys(files: &[DecisionFile]) -> HashSet<String> {
    files
        .iter()
        .flat_map(|file| &file.decisions)
        .flat_map(|decision| [decision.id.clone(), format!("{}_desc", decision.id)])
        .collect()
}

pub(super) fn apply_localization(files: &mut [DecisionFile], values: &HashMap<String, String>) {
    for decision in files.iter_mut().flat_map(|file| &mut file.decisions) {
        if let Some(name) = values.get(&decision.id).filter(|value| !value.is_empty()) {
            decision.name.clone_from(name);
        }
        if let Some(description) = values
            .get(&format!("{}_desc", decision.id))
            .filter(|value| !value.is_empty())
        {
            decision.tooltip = format!("{}\n\n{}", concise(description, 500), decision.tooltip);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_config_entries;

    #[test]
    fn extracts_decisions_in_order_without_treating_category_files_as_decisions() {
        let files = BTreeMap::from([
            (
                "common/decisions/first.txt".into(),
                parse_config_entries(
                    r#"
                group_a = {
                    alpha = { icon = sample cost = 25 available = { flag = yes } }
                    beta = { icon = GFX_direct }
                }
                group_b = { alpha = { complete_effect = { x = 1 } } }
            "#,
                )
                .unwrap(),
            ),
            (
                "common/decisions/categories/meta.txt".into(),
                parse_config_entries("group_a = { visible = { flag = yes } }").unwrap(),
            ),
            (
                "other/decisions.txt".into(),
                parse_config_entries("group = { ignored = { } }").unwrap(),
            ),
        ]);
        let extracted = extract(&files);
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].path, "common/decisions/first.txt");
        let decisions = &extracted[0].decisions;
        assert_eq!(
            decisions.iter().map(|d| d.id.as_str()).collect::<Vec<_>>(),
            ["alpha", "beta", "alpha"]
        );
        assert_eq!(decisions[0].category, "group_a");
        assert_eq!(decisions[2].category, "group_b");
        assert_eq!(decisions[0].icon.as_deref(), Some("GFX_decision_sample"));
        assert_eq!(decisions[1].icon.as_deref(), Some("GFX_direct"));
        assert_eq!(decisions[2].icon, None);
        assert!(decisions[0].tooltip.contains("cost = 25"));
        assert!(decisions[0].tooltip.contains("available ="));
    }

    #[test]
    fn localizes_title_and_description_without_losing_fallback_or_details() {
        let entries = parse_config_entries("group = { alpha = { cost = 3 } beta = { } }").unwrap();
        let mut files = extract(&BTreeMap::from([(
            "common/decisions/sample.txt".into(),
            entries,
        )]));
        let values = HashMap::from([
            ("alpha".into(), "First".into()),
            ("alpha_desc".into(), "Description".into()),
        ]);
        apply_localization(&mut files, &values);
        assert_eq!(files[0].decisions[0].name, "First");
        assert!(files[0].decisions[0].tooltip.contains("Description"));
        assert!(files[0].decisions[0].tooltip.contains("cost = 3"));
        assert_eq!(files[0].decisions[1].name, "beta");
    }

    #[test]
    fn keeps_files_without_decision_objects() {
        let entries = parse_config_entries("group = { value = yes }").unwrap();
        let files = extract(&BTreeMap::from([(
            "common/decisions/empty.txt".into(),
            entries,
        )]));
        assert_eq!(files.len(), 1);
        assert!(files[0].decisions.is_empty());
    }
}
