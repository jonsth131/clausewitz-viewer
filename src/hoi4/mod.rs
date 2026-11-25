pub mod decisions;
pub mod events;
pub mod focus;
pub mod icons;
pub mod ideas;
mod localization;

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

pub fn localize(
    trees: &mut [focus::FocusTree],
    decision_files: &mut [decisions::DecisionFile],
    event_files: &mut [events::EventFile],
    idea_files: &mut [ideas::IdeaFile],
    root: &Path,
    cancelled: &AtomicBool,
) -> Vec<String> {
    let mut keys = focus::localization_keys(trees);
    keys.extend(decisions::localization_keys(decision_files));
    keys.extend(events::localization_keys(event_files));
    keys.extend(ideas::localization_keys(idea_files));
    let (values, errors) = localization::read(&keys, root, cancelled);
    focus::apply_localization(trees, &values);
    decisions::apply_localization(decision_files, &values);
    events::apply_localization(event_files, &values);
    ideas::apply_localization(idea_files, &values);
    errors
}
