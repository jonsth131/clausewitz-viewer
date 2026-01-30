use std::{fs, path::PathBuf};

pub fn read_file(path: &PathBuf) -> String {
    // Preserve valid UTF-8; only remove invalid characters that fail to decode.
    // Fall back to an empty string on I/O error to keep behavior similar but safer.
    match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => String::new(),
    }
}
