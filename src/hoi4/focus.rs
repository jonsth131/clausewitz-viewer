use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::path::Path;
#[cfg(test)]
use std::sync::atomic::AtomicBool;

use crate::parser::{ConfigEntry, ConfigPair, ConfigValue};

use super::concise;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeChoice {
    pub index: usize,
    pub label: String,
}

impl fmt::Display for TreeChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

#[derive(Debug, Clone)]
pub struct Focus {
    pub id: String,
    pub name: String,
    pub tooltip: String,
    pub icon: Option<String>,
    pub position: Option<(i32, i32)>,
    pub prerequisites: Vec<Vec<String>>,
    pub hidden: bool,
}

#[derive(Debug, Clone)]
pub struct FocusTree {
    pub choice: TreeChoice,
    pub focuses: Vec<Focus>,
}

fn pairs(entries: &[ConfigEntry]) -> impl Iterator<Item = &ConfigPair> {
    entries.iter().filter_map(|entry| match entry {
        ConfigEntry::Pair(pair) => Some(pair),
        ConfigEntry::Value(_) => None,
    })
}

fn scalar(value: &ConfigValue) -> Option<&str> {
    match value {
        ConfigValue::Identifier(s) | ConfigValue::String(s) => Some(s),
        _ => None,
    }
}

fn id(entries: &[ConfigEntry]) -> Option<&str> {
    pairs(entries)
        .find(|pair| pair.identifier == "id")
        .and_then(|pair| scalar(&pair.value))
        .filter(|value| !value.is_empty())
}

fn coordinate(entries: &[ConfigEntry], name: &str) -> Option<i32> {
    let value = pairs(entries).find(|pair| pair.identifier == name)?;
    match value.value {
        ConfigValue::Number(number)
            if number.fract() == 0.0 && number >= i32::MIN as f64 && number <= i32::MAX as f64 =>
        {
            Some(number as i32)
        }
        _ => None,
    }
}

fn prerequisite_groups(entries: &[ConfigEntry]) -> Vec<Vec<String>> {
    pairs(entries)
        .filter(|pair| pair.identifier == "prerequisite")
        .filter_map(|pair| match &pair.value {
            ConfigValue::Object(entries) => {
                let ids: Vec<_> = pairs(entries)
                    .filter(|pair| pair.identifier == "focus")
                    .filter_map(|pair| scalar(&pair.value))
                    .map(str::to_owned)
                    .collect();
                (!ids.is_empty()).then_some(ids)
            }
            _ => None,
        })
        .collect()
}

struct PositionSpec {
    offset: Option<(i32, i32)>,
    relative_to: Option<String>,
}

fn resolve_positions(focuses: &mut [Focus], offsets: &[PositionSpec]) {
    let mut ids = HashMap::new();
    for (index, focus) in focuses
        .iter()
        .enumerate()
        .filter(|(_, focus)| !focus.id.is_empty())
    {
        ids.entry(focus.id.as_str())
            .and_modify(|entry| *entry = None)
            .or_insert(Some(index));
    }
    let mut states = vec![0u8; focuses.len()];
    let mut positions: Vec<Option<(i32, i32)>> = vec![None; focuses.len()];
    for start in 0..focuses.len() {
        let mut path = Vec::new();
        let mut current = start;
        loop {
            if states[current] != 0 {
                break;
            }
            states[current] = 1;
            path.push(current);
            match &offsets[current].relative_to {
                Some(reference) => match ids.get(reference.as_str()).copied().flatten() {
                    Some(index) => current = index,
                    None => break,
                },
                None => break,
            }
        }
        while let Some(index) = path.pop() {
            positions[index] = offsets[index].offset.and_then(|(x, y)| {
                let Some(reference) = &offsets[index].relative_to else {
                    return Some((x, y));
                };
                let parent = ids.get(reference.as_str()).copied().flatten()?;
                if states[parent] != 2 {
                    return None;
                }
                let (parent_x, parent_y) = positions[parent]?;
                Some((parent_x.checked_add(x)?, parent_y.checked_add(y)?))
            });
            states[index] = 2;
        }
    }
    for (focus, position) in focuses.iter_mut().zip(positions) {
        focus.position = position;
    }
}

pub fn extract(files: &BTreeMap<String, Vec<ConfigEntry>>) -> Vec<FocusTree> {
    let mut trees = Vec::new();
    let mut labels = HashMap::<String, usize>::new();
    for (path, entries) in files {
        if !Path::new(path).starts_with(Path::new("common/national_focus")) {
            continue;
        }
        for pair in pairs(entries).filter(|pair| pair.identifier == "focus_tree") {
            let ConfigValue::Object(tree_entries) = &pair.value else {
                continue;
            };
            let tree_id = id(tree_entries).unwrap_or("<unnamed tree>");
            let mut focuses = Vec::new();
            let mut offsets = Vec::new();
            for focus in pairs(tree_entries).filter(|pair| pair.identifier == "focus") {
                let ConfigValue::Object(fields) = &focus.value else {
                    continue;
                };
                let focus_id = id(fields).unwrap_or("");
                let name = if focus_id.is_empty() {
                    "<unnamed focus>"
                } else {
                    focus_id
                };
                let mut tooltip = format!("ID: {name}");
                for field in pairs(fields) {
                    let label = match field.identifier.as_str() {
                        "icon" => "Icon",
                        "x" => "X",
                        "y" => "Y",
                        "relative_position_id" => "Relative to",
                        "cost" => "Cost",
                        "prerequisite" => "Prerequisite",
                        "mutually_exclusive" => "Mutually exclusive",
                        "available" => "Available",
                        "bypass" => "Bypass",
                        "completion_reward" => "Completion reward",
                        "hidden" => "Hidden",
                        _ => continue,
                    };
                    let value = field
                        .value
                        .to_string()
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    tooltip.push_str(&format!("\n{label}: {}", concise(&value, 240)));
                }
                tooltip = concise(&tooltip, 1000);
                focuses.push(Focus {
                    id: focus_id.to_owned(),
                    name: name.to_owned(),
                    tooltip,
                    icon: pairs(fields)
                        .find(|pair| pair.identifier == "icon")
                        .and_then(|pair| scalar(&pair.value))
                        .filter(|value| !value.is_empty())
                        .map(str::to_owned),
                    position: None,
                    prerequisites: prerequisite_groups(fields),
                    hidden: pairs(fields).any(|pair| {
                        pair.identifier == "hidden"
                            && pair.sign == "="
                            && matches!(&pair.value, ConfigValue::Identifier(value) if value == "yes")
                    }),
                });
                let position = coordinate(fields, "x").zip(coordinate(fields, "y"));
                let relative = pairs(fields)
                    .find(|pair| pair.identifier == "relative_position_id")
                    .map(|pair| scalar(&pair.value).unwrap_or("").to_owned());
                offsets.push(PositionSpec {
                    offset: position,
                    relative_to: relative,
                });
            }
            resolve_positions(&mut focuses, &offsets);
            let label = format!("{tree_id} ({path})");
            let count = labels.entry(label.clone()).or_default();
            *count += 1;
            let label = if *count > 1 {
                format!("{label} [#{}]", *count)
            } else {
                label
            };
            trees.push(FocusTree {
                choice: TreeChoice {
                    index: trees.len(),
                    label,
                },
                focuses,
            });
        }
    }
    trees
}

pub(super) fn localization_keys(trees: &[FocusTree]) -> HashSet<String> {
    trees
        .iter()
        .flat_map(|tree| &tree.focuses)
        .filter(|focus| !focus.id.is_empty())
        .flat_map(|focus| [focus.id.clone(), format!("{}_desc", focus.id)])
        .collect()
}

pub(super) fn apply_localization(trees: &mut [FocusTree], values: &HashMap<String, String>) {
    for focus in trees
        .iter_mut()
        .flat_map(|tree| &mut tree.focuses)
        .filter(|focus| !focus.id.is_empty())
    {
        if let Some(name) = values.get(&focus.id).filter(|value| !value.is_empty()) {
            focus.name.clone_from(name);
        }
        if let Some(description) = values
            .get(&format!("{}_desc", focus.id))
            .filter(|value| !value.is_empty())
        {
            focus.tooltip = format!("{}\n\n{}", concise(description, 500), focus.tooltip);
        }
    }
}

#[cfg(test)]
fn localize(trees: &mut [FocusTree], root: &Path, cancelled: &AtomicBool) -> Vec<String> {
    let (values, errors) = super::localization::read(&localization_keys(trees), root, cancelled);
    apply_localization(trees, &values);
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_config_entries;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn extracts_scalar_icons_and_ignores_missing_or_non_scalar_values() {
        let files = BTreeMap::from([(
            "common/national_focus/sample.txt".into(),
            parse_config_entries(
                r#"focus_tree = {
                    focus = { id = a icon = GFX_alpha }
                    focus = { id = b icon = "GFX_beta" }
                    focus = { id = c icon = { invalid = yes } }
                    focus = { id = d }
                }"#,
            )
            .unwrap(),
        )]);
        let trees = extract(&files);
        assert_eq!(
            trees[0]
                .focuses
                .iter()
                .map(|focus| focus.icon.as_deref())
                .collect::<Vec<_>>(),
            [Some("GFX_alpha"), Some("GFX_beta"), None, None]
        );
    }

    #[test]
    fn extracts_individual_trees_and_ordered_focuses_with_metadata() {
        let input = r#"
            focus_tree = { id = first focus = { id = alpha prerequisite = { focus = earlier } prerequisite = { focus = other } cost = 5 } focus = { id = beta available = { enabled = yes } } }
            focus_tree = { id = first focus = { id = gamma } }
        "#;
        let files = BTreeMap::from([
            (
                "common/national_focus/sample.txt".into(),
                parse_config_entries(input).unwrap(),
            ),
            (
                "other/sample.txt".into(),
                parse_config_entries(input).unwrap(),
            ),
        ]);
        let trees = extract(&files);
        assert_eq!(trees.len(), 2);
        assert_eq!(
            trees[0].choice.label,
            "first (common/national_focus/sample.txt)"
        );
        assert_eq!(
            trees[0]
                .focuses
                .iter()
                .map(|f| f.id.as_str())
                .collect::<Vec<_>>(),
            ["alpha", "beta"]
        );
        assert_eq!(trees[1].focuses[0].id, "gamma");
        assert_eq!(
            trees[1].choice.label,
            "first (common/national_focus/sample.txt) [#2]"
        );
        assert_eq!(
            trees[0].focuses[0].tooltip.matches("Prerequisite:").count(),
            2
        );
        assert!(trees[0].focuses[0].tooltip.contains("Cost: 5"));
        assert!(trees[0].focuses[1].tooltip.contains("Available:"));
    }

    #[test]
    fn identifies_only_explicit_hidden_focuses() {
        let files = BTreeMap::from([(
            "common/national_focus/sample.txt".into(),
            parse_config_entries(
                "focus_tree = {
                    focus = { id = a hidden = yes }
                    focus = { id = b hidden = no }
                    focus = { id = c visible = { always = no } }
                    focus = { id = d available = { always = no } }
                    focus = { id = e }
                }",
            )
            .unwrap(),
        )]);
        let trees = extract(&files);
        assert_eq!(
            trees[0]
                .focuses
                .iter()
                .map(|focus| focus.hidden)
                .collect::<Vec<_>>(),
            [true, false, false, false, false]
        );
        assert!(trees[0].focuses[0].tooltip.contains("Hidden: yes"));
    }

    #[test]
    fn resolves_chained_positions_and_keeps_prerequisite_groups() {
        let files = BTreeMap::from([(
            "common/national_focus/sample.txt".into(),
            parse_config_entries(
                "focus_tree = {
                    focus = { id = leaf x = -2 y = 3 relative_position_id = middle
                        prerequisite = { focus = root focus = alternate }
                        prerequisite = { focus = middle } }
                    focus = { id = middle x = 1 y = 2 relative_position_id = root }
                    focus = { id = root x = 4 y = 5 }
                    focus = { id = hidden x = 4 y = 5 visible = { always = no } }
                }",
            )
            .unwrap(),
        )]);
        let trees = extract(&files);
        let focuses = &trees[0].focuses;
        assert_eq!(focuses[0].position, Some((3, 10)));
        assert_eq!(focuses[1].position, Some((5, 7)));
        assert_eq!(focuses[2].position, Some((4, 5)));
        assert_eq!(focuses[3].position, Some((4, 5)));
        assert_eq!(
            focuses[0].prerequisites,
            vec![vec!["root", "alternate"], vec!["middle"]]
        );
    }

    #[test]
    fn retains_unresolved_focuses_and_rejects_ambiguous_or_invalid_positions() {
        let files = BTreeMap::from([(
            "common/national_focus/sample.txt".into(),
            parse_config_entries(
                "focus_tree = {
                    focus = { id = cycle_a x = 0 y = 1 relative_position_id = cycle_b }
                    focus = { id = cycle_b x = 0 y = 1 relative_position_id = cycle_a }
                    focus = { id = dependent x = 0 y = 1 relative_position_id = cycle_a }
                    focus = { id = missing x = 1 y = 1 relative_position_id = absent }
                    focus = { id = duplicate x = 1 y = 1 }
                    focus = { id = duplicate x = 2 y = 2 }
                    focus = { id = ambiguous x = 0 y = 0 relative_position_id = duplicate }
                    focus = { id = fractional x = 1.5 y = 1 }
                    focus = { id = no_y x = 1 }
                    focus = { x = 2 y = 3 }
                    focus = { id = edge x = 2147483647 y = 1 }
                    focus = { id = overflow x = 1 y = 0 relative_position_id = edge }
                }",
            )
            .unwrap(),
        )]);
        let trees = extract(&files);
        let focuses = &trees[0].focuses;
        assert_eq!(focuses.len(), 12);
        for index in [0, 1, 2, 3, 6, 7, 8, 11] {
            assert_eq!(focuses[index].position, None, "{}", focuses[index].id);
        }
        assert_eq!(focuses[9].name, "<unnamed focus>");
        assert_eq!(focuses[9].position, Some((2, 3)));
        assert_eq!(focuses[10].position, Some((i32::MAX, 1)));
    }

    #[test]
    fn reads_requested_localization_and_falls_back_on_missing_or_invalid() {
        let root = tempdir().unwrap();
        let folder = root.path().join("localisation/english");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("synthetic.yml"), "\u{feff}l_english:\n alpha:0 \"First \\\"name\\\"\"\n alpha_desc: \"Line one\\nLine two\"\n beta: \"unfinished\n").unwrap();
        let files = BTreeMap::from([(
            "common/national_focus/sample.txt".into(),
            parse_config_entries(
                "focus_tree = { id = sample focus = { id = alpha } focus = { id = beta } }",
            )
            .unwrap(),
        )]);
        let mut trees = extract(&files);
        let errors = localize(&mut trees, root.path(), &AtomicBool::new(false));
        assert_eq!(trees[0].focuses[0].name, "First \"name\"");
        assert!(trees[0].focuses[0].tooltip.contains("Line one\nLine two"));
        assert_eq!(trees[0].focuses[1].name, "beta");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains(":4"));
    }

    #[test]
    fn missing_localization_is_reported_without_losing_focuses() {
        let root = tempdir().unwrap();
        let files = BTreeMap::from([(
            "common/national_focus/sample.txt".into(),
            parse_config_entries("focus_tree = { id = sample focus = { id = alpha } }").unwrap(),
        )]);
        let mut trees = extract(&files);
        assert_eq!(
            localize(&mut trees, root.path(), &AtomicBool::new(false)).len(),
            1
        );
        assert_eq!(trees[0].focuses[0].name, "alpha");
    }

    #[test]
    fn limits_large_tooltip_fields_without_breaking_unicode() {
        let value = "é".repeat(300);
        let input =
            format!("focus_tree = {{ id = sample focus = {{ id = alpha icon = \"{value}\" }} }}");
        let files = BTreeMap::from([(
            "common/national_focus/sample.txt".into(),
            parse_config_entries(&input).unwrap(),
        )]);
        let trees = extract(&files);
        assert!(trees[0].focuses[0]
            .tooltip
            .contains("[truncated; see Files view]"));
    }
}
