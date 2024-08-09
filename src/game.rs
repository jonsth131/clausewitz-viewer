use std::{collections::HashMap, fs, path::PathBuf};

use crate::{
    file::read_file,
    parser::{parse_config_file, ConfigPair},
};

pub fn parse_game(path: &PathBuf) -> HashMap<String, Vec<ConfigPair>> {
    let mut parsed_files = HashMap::new();

    let files = find_txt_files(&path);

    for file in files {
        let file_name = file.file_stem().unwrap().to_str().unwrap().to_string();
        let parsed = parse_file(&file);

        if parsed.is_empty() {
            continue;
        }

        parsed_files.insert(file_name.clone(), parsed);
    }

    parsed_files
}

fn find_txt_files(path: &PathBuf) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_dir() {
            files.append(&mut find_txt_files(&path));
        } else {
            let extension = match path.extension() {
                Some(extension) => extension,
                None => continue,
            };

            if extension != "txt" {
                continue;
            }

            files.push(path);
        }
    }

    files
}

fn parse_file(path: &PathBuf) -> Vec<ConfigPair> {
    let unparsed = read_file(path);
    let parsed = parse_config_file(&unparsed);

    match parsed {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!(
                "Error parsing {}: {}",
                path.file_stem().unwrap().to_str().unwrap(),
                e
            );
            Vec::new()
        }
    }
}
