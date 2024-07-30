use std::{fs, path::PathBuf};

pub fn read_file(path: &PathBuf) -> String {
    fs::read_to_string(path)
        .unwrap()
        .chars()
        .filter(|c| c.is_ascii())
        .collect::<String>()
}
