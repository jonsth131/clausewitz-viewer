use std::borrow::Cow;
use std::fmt::{self, Display, Formatter};

use pest::error::{Error, ErrorVariant};
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "config.pest"]
struct ConfigParser;

#[derive(Debug, PartialEq, Clone)]
pub enum ConfigValue {
    Object(Vec<ConfigEntry>),
    Array(Vec<ConfigValue>),
    Template(String, Vec<ConfigEntry>),
    Empty,
    Expression(String),
    String(String),
    Number(f64),
    Identifier(String),
    Date(String),
    Named(String, Vec<ConfigValue>),
}

impl Display for ConfigValue {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ConfigValue::Object(object) => {
                writeln!(f, "{{")?;
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
            ConfigValue::Template(name, entries) => {
                writeln!(f, "[[{name}]")?;
                for entry in entries {
                    write!(f, "   {entry}")?;
                }
                write!(f, "]")
            }
            ConfigValue::Empty => Ok(()),
            ConfigValue::Expression(expression) => write!(f, "{expression}"),
            ConfigValue::String(string) => {
                write!(f, "\"")?;
                for c in string.chars() {
                    match c {
                        '"' => write!(f, "\\\"")?,
                        '\\' => write!(f, "\\\\")?,
                        '\n' => write!(f, "\\n")?,
                        '\r' => write!(f, "\\r")?,
                        '\t' => write!(f, "\\t")?,
                        '\u{8}' => write!(f, "\\b")?,
                        '\u{c}' => write!(f, "\\f")?,
                        _ => write!(f, "{c}")?,
                    }
                }
                write!(f, "\"")
            }
            ConfigValue::Number(number) => write!(f, "{}", number),
            ConfigValue::Identifier(identifier) => write!(f, "{}", identifier),
            ConfigValue::Date(date) => write!(f, "{date}"),
            ConfigValue::Named(name, values) => {
                write!(f, "{} {{ ", name)?;
                for value in values {
                    write!(f, "{} ", value)?;
                }
                write!(f, "}}")
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

#[derive(Debug, PartialEq, Clone)]
pub enum ConfigEntry {
    Pair(ConfigPair),
    Value(ConfigValue),
}

impl From<ConfigPair> for ConfigEntry {
    fn from(pair: ConfigPair) -> Self {
        Self::Pair(pair)
    }
}

impl Display for ConfigEntry {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ConfigEntry::Pair(pair) => write!(f, "{pair}"),
            ConfigEntry::Value(value) => writeln!(f, "{value}"),
        }
    }
}

impl Display for ConfigPair {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.value == ConfigValue::Empty {
            writeln!(f, "{} {}", self.identifier, self.sign)
        } else {
            writeln!(f, "{} {} {}", self.identifier, self.sign, self.value)
        }
    }
}

type ParseError = Box<Error<Rule>>;

fn conversion_error(pair: &Pair<'_, Rule>, message: impl Into<String>) -> ParseError {
    Box::new(Error::new_from_span(
        ErrorVariant::CustomError {
            message: message.into(),
        },
        pair.as_span(),
    ))
}

fn decode_string(pair: &Pair<'_, Rule>) -> Result<String, ParseError> {
    let text = pair.as_str();
    let mut chars = text[1..text.len() - 1].chars();
    let mut result = String::new();
    while let Some(c) = chars.next() {
        if c != '\\' {
            result.push(c);
            continue;
        }
        result.push(match chars.next() {
            Some('"') => '"',
            Some('\\') => '\\',
            Some('/') => '/',
            Some('b') => '\u{8}',
            Some('f') => '\u{c}',
            Some('n') => '\n',
            Some('r') => '\r',
            Some('t') => '\t',
            _ => return Err(conversion_error(pair, "invalid string escape")),
        });
    }
    Ok(result)
}

fn parse_pair(pair: Pair<'_, Rule>) -> Result<ConfigPair, ParseError> {
    let error = || conversion_error(&pair, "invalid key-value pair");
    let mut inner = pair.clone().into_inner();
    let key = inner.next().ok_or_else(error)?;
    let identifier = if key.as_rule() == Rule::string {
        decode_string(&key)?
    } else {
        key.as_str().to_owned()
    };
    let sign = inner.next().ok_or_else(error)?.as_str().to_owned();
    let value = inner
        .next()
        .map(parse_value)
        .transpose()?
        .unwrap_or(ConfigValue::Empty);
    Ok(ConfigPair {
        identifier,
        sign,
        value,
    })
}

fn parse_scope_entry(entry: Pair<'_, Rule>) -> Result<ConfigEntry, ParseError> {
    let error = || conversion_error(&entry, "invalid scope entry");
    let inner = entry.clone().into_inner().next().ok_or_else(error)?;
    match inner.as_rule() {
        Rule::pair => parse_pair(inner).map(ConfigEntry::Pair),
        _ => parse_value(inner).map(ConfigEntry::Value),
    }
}

fn parse_scope_entries(pair: Pair<'_, Rule>) -> Result<Vec<ConfigEntry>, ParseError> {
    pair.into_inner().map(parse_scope_entry).collect()
}

fn parse_value(pair: Pair<'_, Rule>) -> Result<ConfigValue, ParseError> {
    match pair.as_rule() {
        Rule::object => {
            let entries = parse_scope_entries(pair)?;
            if !entries.is_empty()
                && entries
                    .iter()
                    .all(|entry| matches!(entry, ConfigEntry::Value(_)))
            {
                Ok(ConfigValue::Array(
                    entries
                        .into_iter()
                        .map(|entry| match entry {
                            ConfigEntry::Value(value) => value,
                            ConfigEntry::Pair(_) => unreachable!(),
                        })
                        .collect(),
                ))
            } else {
                Ok(ConfigValue::Object(entries))
            }
        }
        Rule::template => {
            let mut inner = pair.clone().into_inner();
            let error = || conversion_error(&pair, "invalid template block");
            let name = inner.next().ok_or_else(error)?.as_str().to_owned();
            let entries = inner.map(parse_scope_entry).collect::<Result<_, _>>()?;
            Ok(ConfigValue::Template(name, entries))
        }
        Rule::expression => Ok(ConfigValue::Expression(pair.as_str().to_owned())),
        Rule::string => Ok(ConfigValue::String(decode_string(&pair)?)),
        Rule::number => {
            let number: f64 = pair
                .as_str()
                .parse()
                .map_err(|error| conversion_error(&pair, format!("invalid number: {error}")))?;
            if !number.is_finite() {
                return Err(conversion_error(
                    &pair,
                    "number is outside the supported range",
                ));
            }
            Ok(ConfigValue::Number(number))
        }
        Rule::identifier => Ok(ConfigValue::Identifier(pair.as_str().to_owned())),
        Rule::date => Ok(ConfigValue::Date(pair.as_str().to_owned())),
        Rule::named => {
            let error = || conversion_error(&pair, "invalid named value");
            let mut inner = pair.clone().into_inner();
            let name = inner.next().ok_or_else(error)?.as_str().to_owned();
            let array = inner.next().ok_or_else(error)?;
            let values = array
                .into_inner()
                .map(parse_value)
                .collect::<Result<_, _>>()?;
            Ok(ConfigValue::Named(name, values))
        }
        _ => Err(conversion_error(&pair, "unexpected value")),
    }
}

fn close_unclosed_scopes(file: &str) -> Cow<'_, str> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut in_comment = false;
    let mut escaped = false;
    let mut has_unmatched_close = false;

    for character in file.chars() {
        if in_comment {
            if matches!(character, '\r' | '\n') {
                in_comment = false;
            }
            continue;
        }
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        match character {
            '#' => in_comment = true,
            '"' => in_string = true,
            '{' => depth += 1,
            '}' if depth > 0 => depth -= 1,
            '}' => has_unmatched_close = true,
            _ => {}
        }
    }

    if depth > 0 && !has_unmatched_close && !in_string {
        Cow::Owned(format!("{file}\n{}", "}".repeat(depth)))
    } else {
        Cow::Borrowed(file)
    }
}

#[cfg(test)]
pub fn parse_config_file(file: &str) -> Result<Vec<ConfigPair>, Box<Error<Rule>>> {
    let file = file.strip_prefix('\u{feff}').unwrap_or(file);
    let file = close_unclosed_scopes(file);
    ConfigParser::parse(Rule::config, file.as_ref())
        .map_err(Box::new)?
        .filter(|pair| pair.as_rule() != Rule::EOI)
        .map(parse_pair)
        .collect()
}

pub fn parse_config_entries(file: &str) -> Result<Vec<ConfigEntry>, Box<Error<Rule>>> {
    let file = file.strip_prefix('\u{feff}').unwrap_or(file);
    let file = close_unclosed_scopes(file);
    ConfigParser::parse(Rule::config_entries, file.as_ref())
        .map_err(Box::new)?
        .filter(|pair| pair.as_rule() != Rule::EOI)
        .map(parse_scope_entry)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair_at(entries: &[ConfigEntry], index: usize) -> &ConfigPair {
        match &entries[index] {
            ConfigEntry::Pair(pair) => pair,
            ConfigEntry::Value(_) => panic!("expected key-value entry"),
        }
    }

    #[test]
    fn parses_escapes_number_boundaries_and_date_suffixes() {
        let input = r#""quoted\"key" = "a\n\"b\\c" numbers = { 1	2} date = 2099.12.31.5 last = -3"#;
        let parsed = parse_config_file(input).unwrap();
        assert_eq!(parsed[0].identifier, "quoted\"key");
        assert_eq!(parsed[0].value, ConfigValue::String("a\n\"b\\c".into()));
        assert_eq!(parsed[0].value.to_string(), r#""a\n\"b\\c""#);
        assert_eq!(
            parsed[1].value,
            ConfigValue::Array(vec![ConfigValue::Number(1.0), ConfigValue::Number(2.0)])
        );
        assert_eq!(parsed[2].value, ConfigValue::Date("2099.12.31.5".into()));
        assert_eq!(parsed[2].value.to_string(), "2099.12.31.5");
        assert_eq!(parsed[3].value, ConfigValue::Number(-3.0));
    }

    #[test]
    fn parses_named_arrays_and_does_not_split_identifier_prefixes() {
        let parsed =
            parse_config_file("color = rgb { 1 2 3 } token = 123abc empty = rgb {}").unwrap();
        assert_eq!(
            parsed[0].value,
            ConfigValue::Named(
                "rgb".into(),
                vec![
                    ConfigValue::Number(1.0),
                    ConfigValue::Number(2.0),
                    ConfigValue::Number(3.0)
                ]
            )
        );
        assert_eq!(parsed[1].value, ConfigValue::Identifier("123abc".into()));
        assert_eq!(parsed[2].value, ConfigValue::Named("rgb".into(), vec![]));
    }

    #[test]
    fn parses_synthetic_multiline_config_with_mixed_braces() {
        let parsed = parse_config_file(
            r#"
                sample_group =
                {
                    label = "Sample Group"
                    tags = { TAG_A }
                    condition = { enabled = yes }
                    categories = { "type_a" }
                    # A comment containing ignored example syntax.
                    # ignored = { SAMPLE_TAG }
                    fallback = "%d sample item"
                    items =
                    {
                        1 = { "First item" } # first sample item
                        2 = { "Second item" }
                    }
                }
            "#,
        )
        .unwrap();

        let ConfigValue::Object(fields) = &parsed[0].value else {
            panic!("expected top-level object");
        };
        assert_eq!(
            pair_at(fields, 1).value,
            ConfigValue::Array(vec![ConfigValue::Identifier("TAG_A".into())])
        );
        assert_eq!(
            pair_at(fields, 2).value,
            ConfigValue::Object(vec![ConfigPair {
                identifier: "enabled".into(),
                sign: "=".into(),
                value: ConfigValue::Identifier("yes".into()),
            }
            .into()])
        );
        assert_eq!(
            pair_at(fields, 3).value,
            ConfigValue::Array(vec![ConfigValue::String("type_a".into())])
        );
        let ConfigValue::Object(ordered) = &pair_at(fields, 5).value else {
            panic!("expected ordered object");
        };
        assert_eq!(
            pair_at(ordered, 0).value,
            ConfigValue::Array(vec![ConfigValue::String("First item".into())])
        );
    }

    #[test]
    fn parses_synthetic_groups_with_adjacent_string_values() {
        let parsed = parse_config_file(
            r#"
                group_alpha = {
                    categories = { "type_a" "type_b" "type_c" "type_d" }
                    items = {
                        101 = { "First sample item" }
                        102 = { "Second sample item" }
                    }
                }
                group_beta = {
                    categories = { "type_e" }
                    related_group = { group_alpha }
                }
                group_gamma = {
                    label = "Sample Gamma"
                    items = { 10 = { "Tenth sample item" } }
                }
            "#,
        )
        .unwrap();

        assert_eq!(parsed.len(), 3);
        let ConfigValue::Object(group_fields) = &parsed[0].value else {
            panic!("expected group object");
        };
        assert_eq!(
            pair_at(group_fields, 0).value,
            ConfigValue::Array(vec![
                ConfigValue::String("type_a".into()),
                ConfigValue::String("type_b".into()),
                ConfigValue::String("type_c".into()),
                ConfigValue::String("type_d".into()),
            ])
        );
    }

    #[test]
    fn parses_unspaced_comparison_operators() {
        let parsed = parse_config_file("metric>0.24 upper<=3 enabled!=no").unwrap();
        assert_eq!(parsed[0].identifier, "metric");
        assert_eq!(parsed[0].sign, ">");
        assert_eq!(parsed[0].value, ConfigValue::Number(0.24));
        assert_eq!(parsed[1].identifier, "upper");
        assert_eq!(parsed[1].sign, "<=");
        assert_eq!(parsed[1].value, ConfigValue::Number(3.0));
        assert_eq!(parsed[2].identifier, "enabled");
        assert_eq!(parsed[2].sign, "!=");
        assert_eq!(parsed[2].value, ConfigValue::Identifier("no".into()));
    }

    #[test]
    fn parses_explicit_positive_numbers() {
        let parsed = parse_config_file("factor = +0.10").unwrap();
        assert_eq!(parsed[0].value, ConfigValue::Number(0.1));
    }

    #[test]
    fn parses_template_blocks_and_computed_expressions() {
        let parsed = parse_config_file(
            r#"
                outer = {
                    [[!FEATURE]
                        active = yes
                    ]
                    "CHANCE" = @\[( 2 + |score - 1| ) / 4]
                    "FACTOR" = @[ amount * 2 ]
                }
            "#,
        )
        .unwrap();
        let ConfigValue::Object(entries) = &parsed[0].value else {
            panic!("expected outer object");
        };
        let ConfigEntry::Value(ConfigValue::Template(name, body)) = &entries[0] else {
            panic!("expected template block");
        };
        assert_eq!(name, "!FEATURE");
        assert_eq!(pair_at(body, 0).identifier, "active");
        assert_eq!(
            pair_at(body, 0).value,
            ConfigValue::Identifier("yes".into())
        );
        assert_eq!(
            pair_at(entries, 1).value,
            ConfigValue::Expression(r#"@\[( 2 + |score - 1| ) / 4]"#.into())
        );
        assert_eq!(
            pair_at(entries, 2).value,
            ConfigValue::Expression("@[ amount * 2 ]".into())
        );
    }

    #[test]
    fn parses_standalone_and_embedded_bracket_substitutions() {
        let parsed =
            parse_config_file("standalone = [GetValue] embedded = ROOT_[TAG]_suffix").unwrap();
        assert_eq!(
            parsed[0].value,
            ConfigValue::Identifier("[GetValue]".into())
        );
        assert_eq!(
            parsed[1].value,
            ConfigValue::Identifier("ROOT_[TAG]_suffix".into())
        );
    }

    #[test]
    fn parses_mixed_scope_commands_comparisons_and_empty_assignments() {
        let parsed = parse_config_file(
            r#"
                actions = {
                    run_command
                    metric>0.3
                    limit = { enabled = yes }
                    clear_value =
                }
            "#,
        )
        .unwrap();
        let ConfigValue::Object(entries) = &parsed[0].value else {
            panic!("expected mixed scope object");
        };
        assert_eq!(
            entries[0],
            ConfigEntry::Value(ConfigValue::Identifier("run_command".into()))
        );
        assert_eq!(
            entries[1],
            ConfigEntry::Pair(ConfigPair {
                identifier: "metric".into(),
                sign: ">".into(),
                value: ConfigValue::Number(0.3),
            })
        );
        assert_eq!(pair_at(entries, 2).identifier, "limit");
        assert_eq!(pair_at(entries, 3).value, ConfigValue::Empty);
    }

    #[test]
    fn accepts_comment_only_synthetic_files() {
        assert!(parse_config_file("# first comment\n  # final comment")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn parses_top_level_values_in_synthetic_config_entries() {
        let entries =
            parse_config_entries("first_token second_token # comment\nthird_token").unwrap();
        assert_eq!(
            entries,
            [
                ConfigEntry::Value(ConfigValue::Identifier("first_token".into())),
                ConfigEntry::Value(ConfigValue::Identifier("second_token".into())),
                ConfigEntry::Value(ConfigValue::Identifier("third_token".into())),
            ]
        );
    }

    #[test]
    fn closes_unterminated_synthetic_scopes_at_end_of_file() {
        let parsed = parse_config_file("outer = { inner = { value = yes }").unwrap();
        let ConfigValue::Object(outer) = &parsed[0].value else {
            panic!("expected outer object");
        };
        let ConfigValue::Object(inner) = &pair_at(outer, 0).value else {
            panic!("expected nested object");
        };
        assert_eq!(
            pair_at(inner, 0).value,
            ConfigValue::Identifier("yes".into())
        );
    }

    #[test]
    fn closes_unterminated_scope_after_trailing_comment() {
        let parsed = parse_config_file("outer = { value = yes # trailing }").unwrap();
        let ConfigValue::Object(entries) = &parsed[0].value else {
            panic!("expected outer object");
        };
        assert_eq!(
            pair_at(entries, 0).value,
            ConfigValue::Identifier("yes".into())
        );
    }

    #[test]
    fn ignores_braces_in_strings_and_comments_when_closing_scopes() {
        let parsed = parse_config_file("label = \"{\" # }\nouter = { value = yes").unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].value, ConfigValue::String("{".into()));
        assert!(matches!(parsed[1].value, ConfigValue::Object(_)));
    }

    #[test]
    fn parses_files_with_a_leading_utf8_bom_before_comments() {
        let parsed = parse_config_file("\u{feff}# file header\nkey = yes").unwrap();
        assert_eq!(parsed[0].identifier, "key");
        assert_eq!(parsed[0].value, ConfigValue::Identifier("yes".into()));
    }

    #[test]
    fn rejects_nonfinite_numbers_without_panicking() {
        let input = format!("number = {}", "9".repeat(400));
        let error = parse_config_file(&input).unwrap_err().to_string();
        assert!(error.contains("number is outside the supported range"));
    }

    #[test]
    fn test_parse_config_file() {
        let input = r#"
            key1 = "value1"
            key2 = 42
            key3 = yes
            key4 = no
            key5 = 3.15
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
        assert_eq!(cfg[4].value, ConfigValue::Number(3.15));

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
                }
                .into(),
                ConfigPair {
                    identifier: "key9".to_string(),
                    sign: "=".to_string(),
                    value: ConfigValue::Number(84.0)
                }
                .into(),
                ConfigPair {
                    identifier: "key10".to_string(),
                    sign: "=".to_string(),
                    value: ConfigValue::Identifier("yes".to_string())
                }
                .into(),
                ConfigPair {
                    identifier: "key11".to_string(),
                    sign: "=".to_string(),
                    value: ConfigValue::Identifier("no".to_string())
                }
                .into(),
                ConfigPair {
                    identifier: "key12".to_string(),
                    sign: "=".to_string(),
                    value: ConfigValue::Number(3.41)
                }
                .into(),
                ConfigPair {
                    identifier: "key13".to_string(),
                    sign: "=".to_string(),
                    value: ConfigValue::Array(vec![
                        ConfigValue::Number(4.0),
                        ConfigValue::Number(5.0),
                        ConfigValue::Number(6.0)
                    ])
                }
                .into()
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
        let cfg = parse_config_file(r#"key = "invalid\q""#);
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
                }
                .into()])
            }
            .into()])
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
            }
            .into(),
            ConfigPair {
                identifier: "key9".to_string(),
                sign: "=".to_string(),
                value: ConfigValue::Number(84.0),
            }
            .into(),
            ConfigPair {
                identifier: "key12".to_string(),
                sign: "=".to_string(),
                value: ConfigValue::Number(3.41),
            }
            .into(),
            ConfigPair {
                identifier: "key13".to_string(),
                sign: "=".to_string(),
                value: ConfigValue::Array(vec![
                    ConfigValue::Number(4.0),
                    ConfigValue::Number(5.0),
                    ConfigValue::Number(6.0),
                ]),
            }
            .into(),
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
