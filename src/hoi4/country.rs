use core::fmt;
use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
};

use crate::parser::{ConfigPair, ConfigValue};

#[derive(Debug, PartialEq, Default, Clone)]
pub struct Country {
    pub capital: u16,
    pub oob: String,
    pub research_slots: u8,
    pub convoys: u16,
    pub trains: u16,
    pub stability: f32,
    pub war_support: f32,
    pub command_power: u16,
    pub ifs: Vec<ConfigValue>,
    pub recruit_characters: Vec<String>,
    pub created_country_leaders: Vec<CountryLeader>,
    pub start_1939: Vec<ConfigValue>,
    pub politics: Politics,
    pub popularities: Popularities,
    pub technology: Vec<String>,
    pub stockpile: Vec<Equipment>,
    pub ideas: Vec<String>,
    pub variables: Vec<ConfigPair>,
    pub unknown: HashMap<String, ConfigValue>,
}

impl Country {
    pub fn new(pairs: &Vec<ConfigPair>) -> Country {
        let mut country = Country::default();

        for pair in pairs {
            match pair.identifier.as_str() {
                "capital" => country.capital = pair.value.to_u16(),
                "oob" | "OOB" | "set_oob" => country.oob = pair.value.to_string(),
                "set_research_slots" | "add_research_slot" => {
                    country.research_slots = pair.value.to_u8()
                }
                "set_convoys" => country.convoys = pair.value.to_u16(),
                "set_stability" => country.stability = pair.value.to_f32(),
                "if" | "IF" => country.ifs.push(pair.value.clone()),
                "recruit_character" => country.recruit_characters.push(pair.value.to_string()),
                "1939.1.1" => country.start_1939.push(pair.value.clone()),
                "set_politics" => country.politics = Politics::new(&pair.value),
                "set_popularities" => country.popularities = Popularities::new(&pair.value),
                "set_technology" => country.technology = pair.value.get_identifiers_from_object(),
                "add_equipment_to_stockpile" => country.stockpile.push(Equipment::new(&pair.value)),
                "add_ideas" => country.ideas = pair.value.to_strings(),
                "starting_train_buffer" => country.trains = pair.value.to_u16(),
                "set_war_support" => country.war_support = pair.value.to_f32(),
                "set_variable" => country.variables.push(pair.clone()),
                "add_command_power" => country.command_power = pair.value.to_u16(),
                "create_country_leader" => country
                    .created_country_leaders
                    .push(CountryLeader::new(&pair.value)),
                _ => {
                    println!(
                        "Unknown Country identifier: {}: {}",
                        pair.identifier, pair.value
                    );
                    country
                        .unknown
                        .insert(pair.identifier.clone(), pair.value.clone());
                }
            }
        }

        country
    }
}

impl Display for Country {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            r#"
Capital: {}
Oob: {}
Research Slots: {}
Convoys: {}
Trains: {}
Stability: {}
War Support: {}
Command Power: {}
Recruit Characters: {:?}
Created Country Leaders: {:?}
Politics: {}
Popularities: {}
Technology: {:?}
Stockpile: {:?}
Ideas: {:?}
Unknown: {:?}"#,
            self.capital,
            self.oob,
            self.research_slots,
            self.convoys,
            self.trains,
            self.stability,
            self.war_support,
            self.command_power,
            self.recruit_characters,
            self.created_country_leaders,
            self.politics,
            self.popularities,
            self.technology,
            self.stockpile,
            self.ideas,
            self.unknown
        )
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct Politics {
    pub ruling_party: String,
    pub last_election: String,
    pub election_frequency: u8,
    pub elections_allowed: bool,
}

impl Politics {
    pub fn new(config_value: &ConfigValue) -> Politics {
        let mut politics = Politics::default();

        let obj = match config_value {
            ConfigValue::Object(obj) => obj,
            _ => {
                println!("Invalid config value for Politics: {:?}", config_value);
                return politics;
            }
        };

        for pair in obj {
            match pair.identifier.as_str() {
                "ruling_party" => politics.ruling_party = pair.value.to_string(),
                "last_election" => politics.last_election = pair.value.to_string(),
                "election_frequency" => politics.election_frequency = pair.value.to_u8(),
                "elections_allowed" => politics.elections_allowed = pair.value.to_bool(),
                _ => {
                    println!("Unknown Politics identifier: {}", pair.identifier);
                }
            }
        }

        politics
    }
}

impl Display for Politics {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "Ruling Party: {}\nLast Election: {}\nElection Frequency: {}\nElections Allowed: {}",
            self.ruling_party, self.last_election, self.election_frequency, self.elections_allowed
        )
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct Popularities {
    pub communism: u8,
    pub democracy: u8,
    pub fascism: u8,
    pub neutrality: u8,
}

impl Popularities {
    pub fn new(config_value: &ConfigValue) -> Popularities {
        let mut popularities = Popularities::default();

        let obj = match config_value {
            ConfigValue::Object(obj) => obj,
            _ => {
                println!("Invalid config value for Popularities: {:?}", config_value);
                return popularities;
            }
        };

        for pair in obj {
            match pair.identifier.as_str() {
                "communism" => popularities.communism = pair.value.to_u8(),
                "democratic" => popularities.democracy = pair.value.to_u8(),
                "fascism" => popularities.fascism = pair.value.to_u8(),
                "neutrality" => popularities.neutrality = pair.value.to_u8(),
                _ => {
                    println!("Unknown Popularities identifier: {}", pair.identifier);
                }
            }
        }

        popularities
    }
}

impl Display for Popularities {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "Communism: {}\nDemocracy: {}\nFascism: {}\nNeutrality: {}",
            self.communism, self.democracy, self.fascism, self.neutrality
        )
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct Equipment {
    pub name: String,
    pub amount: u16,
    pub producer: String,
}

impl Equipment {
    pub fn new(value: &ConfigValue) -> Equipment {
        let mut equipment = Equipment::default();

        let obj = match value {
            ConfigValue::Object(obj) => obj,
            _ => {
                println!("Invalid config value for Equipment: {:?}", value);
                return equipment;
            }
        };

        for pair in obj {
            match pair.identifier.as_str() {
                "type" => equipment.name = pair.value.to_string(),
                "amount" => equipment.amount = pair.value.to_u16(),
                "producer" => equipment.producer = pair.value.to_string(),
                _ => {
                    println!("Unknown Equipment identifier: {}", pair.identifier);
                }
            }
        }

        equipment
    }
}

impl Display for Equipment {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "Type: {}\nAmount: {}\nProducer {}",
            self.name, self.amount, self.producer
        )
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct CountryLeader {
    pub name: String,
    pub picture: String,
    pub expire: String,
    pub ideology: String,
    pub desc: String,
    pub traits: Vec<String>,
}

impl CountryLeader {
    pub fn new(config_value: &ConfigValue) -> CountryLeader {
        let mut leader = CountryLeader::default();

        let obj = match config_value {
            ConfigValue::Object(obj) => obj,
            _ => {
                println!("Invalid config value for CountryLeader: {:?}", config_value);
                return leader;
            }
        };

        for pair in obj {
            match pair.identifier.as_str() {
                "name" => leader.name = pair.value.to_string(),
                "picture" => leader.picture = pair.value.to_string(),
                "expire" => leader.expire = pair.value.to_string(),
                "ideology" => leader.ideology = pair.value.to_string(),
                "desc" => leader.desc = pair.value.to_string(),
                "traits" => leader.traits = pair.value.to_strings(),
                _ => {
                    println!(
                        "Unknown CountryLeader identifier: {}: {}",
                        pair.identifier, pair.value
                    );
                }
            }
        }

        leader
    }
}

impl Display for CountryLeader {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "Name: {}\nPicture: {}\nExpire: {}\nIdeology: {}\nDescription: {}\nTraits: {:?}",
            self.name, self.picture, self.expire, self.ideology, self.desc, self.traits
        )
    }
}
