use std::sync::Arc;

use iced::widget::{column, container, text, text_editor};
use iced::{Element, Length};

use crate::logging;

pub struct DebugPanel {
    content: text_editor::Content,
    records: Vec<String>,
    version: Option<u64>,
}

impl DebugPanel {
    pub fn new() -> Self {
        Self {
            content: text_editor::Content::new(),
            records: Vec::new(),
            version: None,
        }
    }

    pub fn refresh(&mut self) {
        let Some((version, records)) = logging::snapshot(self.version) else {
            return;
        };
        self.update_snapshot(version, records);
    }

    fn update_snapshot(&mut self, version: u64, records: Vec<String>) {
        if records.starts_with(&self.records) {
            let new_records = &records[self.records.len()..];
            if !new_records.is_empty() {
                let cursor = self.content.cursor();
                let separator = if self.records.is_empty() { "" } else { "\n" };
                let addition = format!("{separator}{}", new_records.join("\n"));
                self.content
                    .perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
                self.content
                    .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                        Arc::new(addition),
                    )));
                self.content.move_to(cursor);
            }
        } else {
            // Keep a selection stable until it is copied or dismissed, even if old records expire.
            if self.content.selection().is_some() {
                return;
            }
            self.content = text_editor::Content::with_text(&records.join("\n"));
        }
        self.records = records;
        self.version = Some(version);
    }

    pub fn interact(&mut self, action: text_editor::Action) {
        if !action.is_edit() {
            self.content.perform(action);
        }
    }

    pub fn view<Message: Clone + 'static>(
        &self,
        on_action: impl Fn(text_editor::Action) -> Message + 'static,
    ) -> Element<'_, Message> {
        container(
            column![
                text("Debug output (Ctrl+`)").size(14),
                text_editor(&self.content)
                    .height(Length::Fill)
                    .on_action(on_action),
            ]
            .spacing(4),
        )
        .padding(8)
        .width(Length::Fill)
        .height(220)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::widget::text_editor::{Action, Edit};

    #[test]
    fn allows_selection_but_ignores_edits_and_preserves_selection_on_append() {
        let mut panel = DebugPanel::new();
        panel.update_snapshot(1, vec!["first".into()]);
        panel.interact(Action::SelectAll);
        panel.interact(Action::Edit(Edit::Delete));

        assert_eq!(panel.content.text(), "first");
        assert_eq!(panel.content.selection().as_deref(), Some("first"));

        panel.update_snapshot(2, vec!["first".into(), "second".into()]);
        assert_eq!(panel.content.text(), "first\nsecond");
        assert_eq!(panel.content.selection().as_deref(), Some("first"));
    }

    #[test]
    fn defers_eviction_while_text_is_selected() {
        let mut panel = DebugPanel::new();
        panel.update_snapshot(1, vec!["first".into(), "second".into()]);
        panel.interact(Action::SelectAll);
        panel.update_snapshot(2, vec!["second".into(), "third".into()]);
        assert_eq!(panel.version, Some(1));
        assert_eq!(panel.content.text(), "first\nsecond");

        panel.interact(Action::Move(text_editor::Motion::DocumentStart));
        panel.update_snapshot(2, vec!["second".into(), "third".into()]);
        assert_eq!(panel.version, Some(2));
        assert_eq!(panel.content.text(), "second\nthird");
    }
}
