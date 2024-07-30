use std::collections::HashMap;

use crate::parser::ConfigValue;

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Stellaris {
    pub variables: HashMap<String, ConfigValue>,
}

pub fn parse_stellaris(path: &std::path::PathBuf) -> Stellaris {
    let mut stellaris = Stellaris::default();

    stellaris.variables = parse_variables(path.clone());

    stellaris
}

fn parse_variables(mut path: std::path::PathBuf) -> HashMap<String, ConfigValue> {
    let mut variables = HashMap::new();

    path.push("common");
    path.push("scripted_variables");

    let paths = std::fs::read_dir(path).unwrap();

    for path in paths {
        let path = path.unwrap().path();
        println!("Parsing {}", path.file_stem().unwrap().to_str().unwrap());
        let unparsed = crate::file::read_file(&path);
        let parsed = crate::parser::parse_config_file(&unparsed);

        match parsed {
            Ok(parsed) => {
                for pair in parsed {
                    variables.insert(pair.identifier, pair.value);
                }
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

    variables
}
