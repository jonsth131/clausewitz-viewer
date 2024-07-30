use std::{collections::HashMap, fs, path::PathBuf};

use country::Country;
use event::Event;
use focustree::FocusTreeBase;

use crate::{file::read_file, parser::parse_config_file};

pub mod country;
pub mod event;
pub mod focustree;

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Hoi4 {
    pub countries: HashMap<String, Country>,
    pub events: HashMap<String, Event>,
    pub focustree: HashMap<String, FocusTreeBase>,
}

pub fn parse_hoi4(path: &PathBuf) -> Hoi4 {
    let mut hoi4 = Hoi4::default();

    hoi4.countries = parse_countries(path.clone());
    //hoi4.events = parse_events(path.clone());
    //hoi4.focustree = parse_focustrees(path.clone());

    hoi4
}

fn parse_countries(mut path: PathBuf) -> HashMap<String, Country> {
    let mut countries = HashMap::new();

    path.push("history");
    path.push("countries");

    let paths = fs::read_dir(path).unwrap();

    for path in paths {
        let path = path.unwrap().path();
        println!("Parsing {}", path.file_stem().unwrap().to_str().unwrap());
        let unparsed = read_file(&path);
        let parsed = parse_config_file(&unparsed);

        match parsed {
            Ok(parsed) => {
                countries.insert(
                    path.file_stem().unwrap().to_str().unwrap().to_string(),
                    Country::new(&parsed),
                );
            }
            Err(e) => {
                eprintln!(
                    "Error parsing {}: {}",
                    path.file_stem().unwrap().to_str().unwrap(),
                    e
                );
                continue;
            }
        }
    }

    countries
}

fn parse_events(mut path: PathBuf) -> HashMap<String, Event> {
    let mut events = HashMap::new();

    path.push("events");

    let paths = fs::read_dir(path).unwrap();

    for path in paths {
        let path = path.unwrap().path();
        println!("Parsing {}", path.file_stem().unwrap().to_str().unwrap());
        let unparsed = read_file(&path);
        let parsed = parse_config_file(&unparsed);

        match parsed {
            Ok(parsed) => {
                events.insert(
                    path.file_stem().unwrap().to_str().unwrap().to_string(),
                    Event::new(&parsed),
                );
            }
            Err(e) => {
                eprintln!(
                    "Error parsing {}: {}",
                    path.file_stem().unwrap().to_str().unwrap(),
                    e
                );
                continue;
            }
        }
    }

    events
}

fn parse_focustrees(mut path: PathBuf) -> HashMap<String, FocusTreeBase> {
    let mut focustrees = HashMap::new();

    path.push("common");
    path.push("national_focus");

    let paths = fs::read_dir(path).unwrap();

    for path in paths {
        let path = path.unwrap().path();
        println!("Parsing {}", path.file_stem().unwrap().to_str().unwrap());
        let unparsed = read_file(&path);
        let parsed = parse_config_file(&unparsed);

        match parsed {
            Ok(parsed) => {
                focustrees.insert(
                    path.file_stem().unwrap().to_str().unwrap().to_string(),
                    FocusTreeBase::new(&parsed),
                );
            }
            Err(e) => {
                eprintln!(
                    "Error parsing {}: {}",
                    path.file_stem().unwrap().to_str().unwrap(),
                    e
                );
                continue;
            }
        }
    }

    focustrees
}
