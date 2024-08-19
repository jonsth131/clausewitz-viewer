use std::fmt::{self, Display, Formatter};

use pest::error::Error;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "config.pest"]
struct ConfigParser;

#[derive(Debug, PartialEq, Clone)]
pub enum ConfigValue {
    Object(Vec<ConfigPair>),
    Array(Vec<ConfigValue>),
    String(String),
    Number(f64),
    Identifier(String),
    Date(u16, u8, u8),
    Named(String, Vec<ConfigValue>),
}

impl Display for ConfigValue {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ConfigValue::Object(object) => {
                write!(f, "{{\n")?;
                for pair in object {
                    write!(f, "   {}", pair)?;
                }
                write!(f, "}}")
            }
            ConfigValue::Array(array) => {
                write!(f, "{{ ")?;
                for value in array {
                    write!(f, "{} ", value)?;
                }
                write!(f, "}}")
            }
            ConfigValue::String(string) => write!(f, "\"{}\"", string),
            ConfigValue::Number(number) => write!(f, "{}", number),
            ConfigValue::Identifier(identifier) => write!(f, "{}", identifier),
            ConfigValue::Date(year, month, day) => write!(f, "{}.{}.{}", year, month, day),
            ConfigValue::Named(name, values) => {
                write!(f, "{} ", name)?;
                for value in values {
                    write!(f, "{} ", value)?;
                }
                write!(f, "")
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ConfigPair {
    pub identifier: String,
    pub sign: String,
    pub value: ConfigValue,
}

impl Display for ConfigPair {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} {} {}\n", self.identifier, self.sign, self.value)
    }
}

pub fn parse_config_file(file: &str) -> Result<Vec<ConfigPair>, Error<Rule>> {
    let cfg = ConfigParser::parse(Rule::config, file)?;

    use pest::iterators::Pair;

    fn parse_pair(pair: Pair<Rule>) -> ConfigPair {
        match pair.as_rule() {
            Rule::pair => {
                let mut inner_rules = pair.into_inner();
                let identifier = inner_rules.next().unwrap().as_str().to_owned();
                let sign = inner_rules.next().unwrap().as_str().to_owned();
                let value = parse_value(inner_rules.next().unwrap());
                ConfigPair {
                    identifier,
                    sign,
                    value,
                }
            }
            _ => unreachable!(),
        }
    }

    fn parse_value(pair: Pair<Rule>) -> ConfigValue {
        match pair.as_rule() {
            Rule::object => {
                let mut inner_rules = pair.into_inner();
                let mut object = Vec::new();
                while let Some(pair) = inner_rules.next() {
                    object.push(parse_pair(pair));
                }
                ConfigValue::Object(object)
            }
            Rule::array => {
                let mut inner_rules = pair.into_inner();
                let mut array = Vec::new();
                while let Some(pair) = inner_rules.next() {
                    array.push(parse_value(pair));
                }
                ConfigValue::Array(array)
            }
            Rule::string => {
                ConfigValue::String(pair.into_inner().next().unwrap().as_str().to_owned())
            }
            Rule::number => ConfigValue::Number(pair.as_str().trim().parse().unwrap()),
            Rule::identifier => ConfigValue::Identifier(pair.as_str().to_owned()),
            Rule::date => {
                let date: Vec<&str> = pair.as_str().split(".").collect();

                if date.len() < 3 {
                    panic!("Invalid date format");
                }

                let year = date[0].parse().unwrap();
                let month = date[1].parse().unwrap();
                let day = date[2].parse().unwrap();

                ConfigValue::Date(year, month, day)
            }
            Rule::named => {
                let mut inner_rules = pair.into_inner();
                let name = inner_rules.next().unwrap().as_str().to_owned();
                let mut values = Vec::new();
                while let Some(pair) = inner_rules.next() {
                    values.push(parse_value(pair));
                }
                ConfigValue::Named(name, values)
            }
            _ => unreachable!(),
        }
    }

    let result: Vec<ConfigPair> = cfg
        .filter(|pair| pair.as_rule() != Rule::EOI)
        .map(parse_pair)
        .collect();

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config_file() {
        let input = r#"
            key1 = "value1"
            key2 = 42
            key3 = yes
            key4 = no
            key5 = 3.14
            key6 = { 1 2 3 }
            key7 = {
                key8 = "value8"
                key9 = 84
                key10 = yes
                key11 = no
                key12 = 3.41
                key13 = { 4 5 6 }
            }
            key14 = identifier_123
            key15 < 1
        "#;

        let cfg = parse_config_file(input).unwrap();

        assert_eq!(cfg.len(), 9);

        assert_eq!(cfg[0].identifier, "key1");
        assert_eq!(cfg[0].value, ConfigValue::String("value1".to_string()));

        assert_eq!(cfg[1].identifier, "key2");
        assert_eq!(cfg[1].value, ConfigValue::Number(42.0));

        assert_eq!(cfg[2].identifier, "key3");
        assert_eq!(cfg[2].value, ConfigValue::Identifier("yes".to_string()));

        assert_eq!(cfg[3].identifier, "key4");
        assert_eq!(cfg[3].value, ConfigValue::Identifier("no".to_string()));

        assert_eq!(cfg[4].identifier, "key5");
        assert_eq!(cfg[4].value, ConfigValue::Number(3.14));

        assert_eq!(cfg[5].identifier, "key6");
        assert_eq!(
            cfg[5].value,
            ConfigValue::Array(vec![
                ConfigValue::Number(1.0),
                ConfigValue::Number(2.0),
                ConfigValue::Number(3.0)
            ])
        );

        assert_eq!(cfg[6].identifier, "key7");
        assert_eq!(
            cfg[6].value,
            ConfigValue::Object(vec![
                ConfigPair {
                    identifier: "key8".to_string(),
                    sign: "=".to_string(),
                    value: ConfigValue::String("value8".to_string())
                },
                ConfigPair {
                    identifier: "key9".to_string(),
                    sign: "=".to_string(),
                    value: ConfigValue::Number(84.0)
                },
                ConfigPair {
                    identifier: "key10".to_string(),
                    sign: "=".to_string(),
                    value: ConfigValue::Identifier("yes".to_string())
                },
                ConfigPair {
                    identifier: "key11".to_string(),
                    sign: "=".to_string(),
                    value: ConfigValue::Identifier("no".to_string())
                },
                ConfigPair {
                    identifier: "key12".to_string(),
                    sign: "=".to_string(),
                    value: ConfigValue::Number(3.41)
                },
                ConfigPair {
                    identifier: "key13".to_string(),
                    sign: "=".to_string(),
                    value: ConfigValue::Array(vec![
                        ConfigValue::Number(4.0),
                        ConfigValue::Number(5.0),
                        ConfigValue::Number(6.0)
                    ])
                }
            ])
        );

        assert_eq!(cfg[7].identifier, "key14");
        assert_eq!(
            cfg[7].value,
            ConfigValue::Identifier("identifier_123".to_string())
        );

        assert_eq!(cfg[8].identifier, "key15");
        assert_eq!(cfg[8].sign, "<");
        assert_eq!(cfg[8].value, ConfigValue::Number(1.0));
    }

    #[test]
    fn test_parse_config_file_error() {
        let input = r#"
            key1 = "value1"
            key2 = 42
            key3 = yes
            key4 = no
            key5 = 3.14
            key6 = { 1 2 3 }
            key7 = {
                key8 = "value8"
                key9 = 84
                key10 = yes
                key11 = no
                key12 = 3.41
                key13 = { 4 5 6 }
        "#;

        let cfg = parse_config_file(input);
        assert!(cfg.is_err());
    }

    #[test]
    fn test_parse_config_file_empty() {
        let input = r#""#;

        let cfg = parse_config_file(input);
        assert!(cfg.is_ok());
        assert_eq!(cfg.unwrap().len(), 0);
    }

    #[test]
    fn test_parse_config_file_empty_line() {
        let input = r#"
        "#;

        let cfg = parse_config_file(input);
        assert!(cfg.is_ok());
        assert_eq!(cfg.unwrap().len(), 0);
    }

    #[test]
    fn test_parse_config_file_object_in_object() {
        let input = r#"
            key1 = {
                key2 = {
                    key3 = "value3"
                }
            }
        "#;

        let cfg = parse_config_file(input).unwrap();

        assert_eq!(cfg.len(), 1);

        assert_eq!(cfg[0].identifier, "key1");
        assert_eq!(
            cfg[0].value,
            ConfigValue::Object(vec![ConfigPair {
                identifier: "key2".to_string(),
                sign: "=".to_string(),
                value: ConfigValue::Object(vec![ConfigPair {
                    identifier: "key3".to_string(),
                    sign: "=".to_string(),
                    value: ConfigValue::String("value3".to_string())
                }])
            }])
        );
    }

    #[test]
    fn test_parse_config_file_comment() {
        let input = r#"
            key1 = "value1" # comment
            # key2 = "value2"
        "#;

        let cfg = parse_config_file(input).unwrap();

        assert_eq!(cfg.len(), 1);

        assert_eq!(cfg[0].identifier, "key1");
        assert_eq!(cfg[0].value, ConfigValue::String("value1".to_string()));
    }

    #[test]
    fn test_configvalue_display() {
        let value = ConfigValue::String("value1".to_string());
        assert_eq!(format!("{}", value), "\"value1\"");

        let value = ConfigValue::Number(42.0);
        assert_eq!(format!("{}", value), "42");

        let value = ConfigValue::Identifier("identifier_123".to_string());
        assert_eq!(format!("{}", value), "identifier_123");

        let value = ConfigValue::Array(vec![
            ConfigValue::Number(1.0),
            ConfigValue::Number(2.0),
            ConfigValue::Number(3.0),
        ]);
        assert_eq!(format!("{}", value), "{ 1 2 3 }");

        let value = ConfigValue::Object(vec![
            ConfigPair {
                identifier: "key8".to_string(),
                sign: "=".to_string(),
                value: ConfigValue::String("value8".to_string()),
            },
            ConfigPair {
                identifier: "key9".to_string(),
                sign: "=".to_string(),
                value: ConfigValue::Number(84.0),
            },
            ConfigPair {
                identifier: "key12".to_string(),
                sign: "=".to_string(),
                value: ConfigValue::Number(3.41),
            },
            ConfigPair {
                identifier: "key13".to_string(),
                sign: "=".to_string(),
                value: ConfigValue::Array(vec![
                    ConfigValue::Number(4.0),
                    ConfigValue::Number(5.0),
                    ConfigValue::Number(6.0),
                ]),
            },
        ]);
        assert_eq!(
            format!("{}", value),
            "{\n   key8 = \"value8\"\n   key9 = 84\n   key12 = 3.41\n   key13 = { 4 5 6 }\n}"
        );
    }

    #[test]
    fn test_configpair_display() {
        let pair = ConfigPair {
            identifier: "key1".to_string(),
            sign: "=".to_string(),
            value: ConfigValue::String("value1".to_string()),
        };
        assert_eq!(format!("{}", pair), "key1 = \"value1\"\n");
    }
}
