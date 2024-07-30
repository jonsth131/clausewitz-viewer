use crate::parser::{ConfigPair, ConfigValue};

#[derive(Debug, PartialEq, Default, Clone)]
pub struct FocusTreeBase {
    pub focus_tree: FocusTree,
    pub shared_focus: Vec<ConfigValue>,
    pub search_filter_prios: Vec<ConfigValue>,
    pub joint_focus: Vec<ConfigValue>,
    pub style: Vec<ConfigValue>,
}

impl FocusTreeBase {
    pub fn new(config_value: &Vec<ConfigPair>) -> FocusTreeBase {
        let mut focus_tree = FocusTreeBase::default();

        for pair in config_value {
            match pair.identifier.as_str() {
                "focus_tree" => focus_tree.focus_tree = FocusTree::new(&pair.value),
                "shared_focus" => focus_tree.shared_focus.push(pair.value.clone()),
                "search_filter_prios" => focus_tree.search_filter_prios.push(pair.value.clone()),
                "joint_focus" => focus_tree.joint_focus.push(pair.value.clone()),
                "style" => focus_tree.style.push(pair.value.clone()),
                _ => {
                    println!(
                        "Unknown FocusTreeBase identifier: {}: {}",
                        pair.identifier, pair.value
                    );
                }
            }
        }

        focus_tree
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct FocusTree {
    pub id: String,
    pub country: Option<ConfigValue>,
    pub default: bool,
    pub initial_show_position: Option<ConfigValue>,
    pub continuous_focus_position: Option<ConfigValue>,
    pub focus: Vec<Focus>,
    pub shared_focus: Vec<ConfigValue>,
    pub reset_on_civilwar: bool,
}

impl FocusTree {
    pub fn new(config_value: &ConfigValue) -> FocusTree {
        let mut focus_tree = FocusTree::default();

        let obj = match config_value {
            ConfigValue::Object(obj) => obj,
            _ => {
                println!("Invalid config value for FocusTree: {:?}", config_value);
                return focus_tree;
            }
        };

        for pair in obj {
            match pair.identifier.as_str() {
                "id" => focus_tree.id = pair.value.to_string(),
                "country" => focus_tree.country = Some(pair.value.clone()),
                "default" => focus_tree.default = pair.value.to_bool(),
                "initial_show_position" => {
                    focus_tree.initial_show_position = Some(pair.value.clone())
                }
                "continuous_focus_position" => {
                    focus_tree.continuous_focus_position = Some(pair.value.clone())
                }
                "focus" => focus_tree.focus.push(Focus::new(&pair.value)),
                "shared_focus" => focus_tree.shared_focus.push(pair.value.clone()),
                "reset_on_civilwar" => focus_tree.reset_on_civilwar = pair.value.to_bool(),
                _ => {
                    println!(
                        "Unknown FocusTree identifier: {}: {}",
                        pair.identifier, pair.value
                    );
                }
            }
        }

        focus_tree
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct Focus {
    pub id: String,
    pub icon: String,
    pub x: i32,
    pub y: i32,
    pub cost: u8,
    pub available: Option<ConfigValue>,
    pub ai_will_do: Option<ConfigValue>,
    pub bypass: Option<ConfigValue>,
    pub completion_reward: Option<ConfigValue>,
    pub search_filters: Vec<String>,
    pub available_if_capitulated: bool,
    pub continue_if_invalid: bool,
    pub cancel_if_invalid: bool,
    pub relative_position_id: String,
    pub prerequisite: Option<ConfigValue>,
    pub allow_branch: Option<ConfigValue>,
    pub offset: Option<ConfigValue>,
    pub mutually_exclusive: Option<ConfigValue>,
    pub complete_tooltip: Option<ConfigValue>,
    pub will_lead_to_war_with: String,
    pub cancel: Option<ConfigValue>,
    pub dynamic: bool,
    pub select_effect: Vec<ConfigValue>,
    pub text: String,
    pub historical_ai: Vec<ConfigValue>,
    pub cancelable: bool,
}

impl Focus {
    pub fn new(config_value: &ConfigValue) -> Focus {
        let mut focus = Focus::default();

        let obj = match config_value {
            ConfigValue::Object(obj) => obj,
            _ => {
                println!("Invalid config value for Focus: {:?}", config_value);
                return focus;
            }
        };

        for pair in obj {
            match pair.identifier.as_str() {
                "id" => focus.id = pair.value.to_string(),
                "icon" => focus.icon = pair.value.to_string(),
                "x" => focus.x = pair.value.to_i32(),
                "y" => focus.y = pair.value.to_i32(),
                "cost" => focus.cost = pair.value.to_u8(),
                "available" => focus.available = Some(pair.value.clone()),
                "ai_will_do" => focus.ai_will_do = Some(pair.value.clone()),
                "bypass" | "Bypass" => focus.bypass = Some(pair.value.clone()),
                "completion_reward" => focus.completion_reward = Some(pair.value.clone()),
                "search_filters" => {
                    focus.search_filters = pair.value.to_strings();
                }
                "available_if_capitulated" => focus.available_if_capitulated = pair.value.to_bool(),
                "continue_if_invalid" => focus.continue_if_invalid = pair.value.to_bool(),
                "cancel_if_invalid" => focus.cancel_if_invalid = pair.value.to_bool(),
                "relative_position_id" => focus.relative_position_id = pair.value.to_string(),
                "prerequisite" => focus.prerequisite = Some(pair.value.clone()),
                "allow_branch" => focus.allow_branch = Some(pair.value.clone()),
                "offset" => focus.offset = Some(pair.value.clone()),
                "mutually_exclusive" => focus.mutually_exclusive = Some(pair.value.clone()),
                "complete_tooltip" => focus.complete_tooltip = Some(pair.value.clone()),
                "will_lead_to_war_with" => focus.will_lead_to_war_with = pair.value.to_string(),
                "cancel" => focus.cancel = Some(pair.value.clone()),
                "dynamic" => focus.dynamic = pair.value.to_bool(),
                "select_effect" => focus.select_effect.push(pair.value.clone()),
                "text" => focus.text = pair.value.to_string(),
                "historical_ai" => focus.historical_ai.push(pair.value.clone()),
                "cancelable" => focus.cancelable = pair.value.to_bool(),
                _ => {
                    println!(
                        "Unknown Focus identifier: {}: {}",
                        pair.identifier, pair.value
                    );
                }
            }
        }

        focus
    }
}
