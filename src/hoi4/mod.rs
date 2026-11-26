pub mod characters;
pub mod decisions;
pub mod events;
pub mod focus;
pub mod ideas;
mod script;
pub mod technologies;

use std::path::Path;
use std::sync::atomic::AtomicBool;

fn concise(value: &str, limit: usize) -> String {
    let mut chars = value.chars();
    let prefix: String = chars.by_ref().take(limit).collect();
    if chars.next().is_some() {
        format!("{prefix}... [truncated; see Files view]")
    } else {
        prefix
    }
}

pub struct LocalizationTargets<'a> {
    pub trees: &'a mut [focus::FocusTree],
    pub decisions: &'a mut [decisions::DecisionFile],
    pub events: &'a mut [events::EventFile],
    pub ideas: &'a mut [ideas::IdeaFile],
    pub technologies: &'a mut [technologies::TechnologyFile],
    pub characters: &'a mut [characters::CharacterFile],
}

pub fn localize(
    targets: LocalizationTargets<'_>,
    root: &Path,
    cancelled: &AtomicBool,
) -> Vec<String> {
    let mut keys = focus::localization_keys(targets.trees);
    keys.extend(decisions::localization_keys(targets.decisions));
    keys.extend(events::localization_keys(targets.events));
    keys.extend(ideas::localization_keys(targets.ideas));
    keys.extend(technologies::localization_keys(targets.technologies));
    keys.extend(characters::localization_keys(targets.characters));
    let (values, errors) = crate::common::localization::read(&keys, root, cancelled);
    focus::apply_localization(targets.trees, &values);
    decisions::apply_localization(targets.decisions, &values);
    events::apply_localization(targets.events, &values);
    ideas::apply_localization(targets.ideas, &values);
    technologies::apply_localization(targets.technologies, &values);
    characters::apply_localization(targets.characters, &values);
    errors
}
