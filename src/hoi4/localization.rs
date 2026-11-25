use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

// Clausewitz localization resembles YAML but permits numeric versions after ':'.
fn localization_line(line: &str) -> Option<(&str, Result<String, ()>)> {
    let line = line.trim().trim_start_matches('\u{feff}');
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let (key, rest) = line.split_once(':')?;
    let key = key.trim();
    if key.is_empty() || key.chars().any(char::is_whitespace) {
        return None;
    }
    let rest = rest.trim_start();
    let rest = rest
        .trim_start_matches(|c: char| c.is_ascii_digit())
        .trim_start();
    let Some(rest) = rest.strip_prefix('"') else {
        return Some((key, Err(())));
    };
    let mut value = String::new();
    let mut chars = rest.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => return Some((key, Ok(value))),
            '\\' => match chars.next() {
                Some('n') => value.push('\n'),
                Some('"') => value.push('"'),
                Some('\\') => value.push('\\'),
                Some(other) => {
                    value.push('\\');
                    value.push(other);
                }
                None => return Some((key, Err(()))),
            },
            _ => value.push(ch),
        }
    }
    Some((key, Err(())))
}

pub(super) fn read(
    keys: &HashSet<String>,
    root: &Path,
    cancelled: &AtomicBool,
) -> (HashMap<String, String>, Vec<String>) {
    if keys.is_empty() {
        return (HashMap::new(), Vec::new());
    }
    let mut errors = Vec::new();
    let mut paths = Vec::new();
    let mut directories = vec![root.join("localisation/english")];
    while let Some(directory) = directories.pop() {
        if cancelled.load(Ordering::Relaxed) {
            return (HashMap::new(), errors);
        }
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) => {
                errors.push(format!(
                    "Could not read localization {}: {error}",
                    directory.display()
                ));
                continue;
            }
        };
        for entry in entries {
            match entry {
                Ok(entry) => match entry.file_type() {
                    Ok(kind) if kind.is_dir() => directories.push(entry.path()),
                    Ok(kind)
                        if kind.is_file()
                            && entry
                                .path()
                                .extension()
                                .is_some_and(|ext| ext.eq_ignore_ascii_case("yml")) =>
                    {
                        paths.push(entry.path())
                    }
                    Ok(_) => {}
                    Err(error) => errors.push(format!(
                        "Could not inspect localization {}: {error}",
                        entry.path().display()
                    )),
                },
                Err(error) => errors.push(format!(
                    "Could not read localization directory {}: {error}",
                    directory.display()
                )),
            }
        }
    }
    paths.sort();
    let mut values = HashMap::new();
    for path in paths {
        if cancelled.load(Ordering::Relaxed) {
            return (values, errors);
        }
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) => {
                errors.push(format!(
                    "Could not read localization {}: {error}",
                    path.display()
                ));
                continue;
            }
        };
        for (line_number, line) in contents.lines().enumerate() {
            if let Some((key, value)) = localization_line(line) {
                if keys.contains(key) {
                    match value {
                        Ok(value) => {
                            values.insert(key.to_owned(), value);
                        }
                        Err(()) => errors.push(format!(
                            "Invalid localization {}:{}",
                            path.display(),
                            line_number + 1
                        )),
                    }
                }
            }
        }
    }
    (values, errors)
}
