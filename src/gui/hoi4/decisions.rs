use std::collections::HashMap;

use iced::widget::{column, container, image, pick_list, row, scrollable, text, tooltip, Column};
use iced::{Alignment, Element, Length};

use crate::hoi4::decisions::{Decision, DecisionFile};

#[derive(Debug, Clone)]
pub enum Message {
    SelectFile(String),
}

#[derive(Debug, Default)]
pub struct DecisionBrowser {
    files: Vec<DecisionFile>,
    choices: Vec<String>,
    selected: Option<String>,
    icons: HashMap<String, image::Handle>,
}

impl DecisionBrowser {
    pub fn new(files: Vec<DecisionFile>, icons: HashMap<String, image::Handle>) -> Self {
        let choices = files.iter().map(|file| file.path.clone()).collect();
        Self {
            files,
            choices,
            selected: None,
            icons,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::SelectFile(path) if self.choices.contains(&path) => self.selected = Some(path),
            Message::SelectFile(_) => {}
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let mut content = column![text("Game: Hearts of Iron IV")].spacing(10);
        if self.choices.is_empty() {
            return container(content.push(text("No decision files were loaded.")))
                .padding(20)
                .into();
        }
        content = content.push(
            pick_list(
                &self.choices[..],
                self.selected.as_ref(),
                Message::SelectFile,
            )
            .placeholder("Choose a decision file"),
        );
        if let Some(file) = self
            .files
            .iter()
            .find(|file| Some(&file.path) == self.selected.as_ref())
        {
            let mut list = Column::new().spacing(6);
            if file.decisions.is_empty() {
                list = list.push(text("No decisions defined in this file."));
            }
            let mut category = None;
            for decision in &file.decisions {
                if category != Some(decision.category.as_str()) {
                    list = list.push(text(&decision.category).size(18));
                    category = Some(&decision.category);
                }
                list = list.push(decision_row(decision, &self.icons));
            }
            content = content.push(scrollable(list).height(Length::Fill).width(Length::Fill));
        }
        container(content).padding(20).height(Length::Fill).into()
    }
}

fn decision_row<'a>(
    decision: &'a Decision,
    icons: &HashMap<String, image::Handle>,
) -> Element<'a, Message> {
    let mut content = row![].spacing(10).align_y(Alignment::Center);
    if let Some(icon) = decision.icon.as_ref().and_then(|name| icons.get(name)) {
        content = content.push(
            image(icon.clone())
                .width(48)
                .height(48)
                .content_fit(iced::ContentFit::Contain),
        );
    }
    content = content.push(text(&decision.name));
    tooltip(
        container(content)
            .padding(8)
            .width(Length::Fill)
            .style(container::rounded_box),
        container(text(&decision.tooltip))
            .padding(10)
            .max_width(500)
            .style(container::rounded_box),
        tooltip::Position::FollowCursor,
    )
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hoi4::decisions, parser::parse_config_entries};
    use std::collections::BTreeMap;

    #[test]
    fn selects_decision_files_and_keeps_fallback_rows() {
        let parsed =
            parse_config_entries("group = { alpha = { icon = test } beta = { } }").unwrap();
        let files = decisions::extract(&BTreeMap::from([(
            "common/decisions/sample.txt".into(),
            parsed,
        )]));
        let mut browser = DecisionBrowser::new(files, HashMap::new());
        assert!(browser.selected.is_none());
        let _ = browser.view();
        browser.update(Message::SelectFile("unknown".into()));
        assert!(browser.selected.is_none());
        browser.update(Message::SelectFile("common/decisions/sample.txt".into()));
        assert_eq!(
            browser.selected.as_deref(),
            Some("common/decisions/sample.txt")
        );
        let _ = browser.view();
        assert_eq!(browser.files[0].decisions[0].name, "alpha");
    }

    #[test]
    fn creates_rows_with_loaded_and_missing_icons() {
        let parsed =
            parse_config_entries("group = { alpha = { icon = test } beta = { icon = absent } }")
                .unwrap();
        let files = decisions::extract(&BTreeMap::from([(
            "common/decisions/sample.txt".into(),
            parsed,
        )]));
        let icons = HashMap::from([(
            "GFX_decision_test".into(),
            image::Handle::from_rgba(1, 1, vec![1, 2, 3, 255]),
        )]);
        let mut browser = DecisionBrowser::new(files, icons);
        browser.update(Message::SelectFile("common/decisions/sample.txt".into()));
        assert!(browser
            .icons
            .contains_key(browser.files[0].decisions[0].icon.as_ref().unwrap()));
        assert!(!browser
            .icons
            .contains_key(browser.files[0].decisions[1].icon.as_ref().unwrap()));
        let _ = browser.view();
    }

    #[test]
    fn lays_out_icon_to_the_left_of_the_decision_title() {
        use iced::advanced::graphics::core::renderer::Headless;
        use iced::advanced::{layout, widget::Tree, Layout};
        use iced::{Renderer, Size};

        let parsed = parse_config_entries("group = { alpha = { icon = test } }").unwrap();
        let files = decisions::extract(&BTreeMap::from([(
            "common/decisions/sample.txt".into(),
            parsed,
        )]));
        let icons = HashMap::from([(
            "GFX_decision_test".into(),
            image::Handle::from_rgba(1, 1, vec![1, 2, 3, 255]),
        )]);
        let mut row = decision_row(&files[0].decisions[0], &icons);
        let renderer = iced::futures::executor::block_on(<Renderer as Headless>::new(
            iced::Font::default(),
            16.into(),
            Some("tiny-skia"),
        ))
        .expect("software renderer");
        let mut tree = Tree::new(row.as_widget());
        let node = row.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, Size::new(500.0, 100.0)),
        );
        let container = Layout::new(&node);
        let row = container.children().next().unwrap();
        let mut children = row.children();
        let icon = children.next().unwrap().bounds();
        let title = children.next().unwrap().bounds();
        assert_eq!(icon.width, 48.0);
        assert!(icon.x + icon.width <= title.x);
    }
}
