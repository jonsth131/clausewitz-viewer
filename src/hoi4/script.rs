use crate::parser::{ConfigEntry, ConfigPair, ConfigValue};

pub fn pairs(entries: &[ConfigEntry]) -> impl Iterator<Item = &ConfigPair> {
    entries.iter().filter_map(|entry| match entry {
        ConfigEntry::Pair(pair) => Some(pair),
        ConfigEntry::Value(_) => None,
    })
}

pub fn scalar(value: &ConfigValue) -> Option<&str> {
    match value {
        ConfigValue::Identifier(value) | ConfigValue::String(value) if !value.is_empty() => {
            Some(value)
        }
        _ => None,
    }
}

pub fn static_sprite(value: &str) -> Option<&str> {
    (value.starts_with("GFX_") && !value.contains(['[', '$'])).then_some(value)
}
