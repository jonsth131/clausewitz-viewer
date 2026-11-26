use iced::theme::Theme;
use iced::widget::{button, column, container, row, text_editor};
use iced::{application, keyboard, Alignment, Element, Font, Length, Subscription, Task};
use std::path::PathBuf;
use std::time::Duration;

mod common;
mod data_view;
mod debug_panel;
mod hoi4;
mod stellaris;

pub fn run() -> iced::Result {
    application(
        ClausewitzViewer::new,
        ClausewitzViewer::update,
        ClausewitzViewer::view,
    )
    .title("Clausewitz Viewer")
    .default_font(Font::MONOSPACE)
    .theme(|_: &ClausewitzViewer| Theme::Dark)
    .subscription(ClausewitzViewer::subscription)
    .run()
}

#[derive(Debug, Default)]
enum View {
    #[default]
    Default,
    DataView(Box<data_view::DataView>),
}

struct ClausewitzViewer {
    view: View,
    debug_panel: debug_panel::DebugPanel,
    debug_visible: bool,
}

#[derive(Debug, Clone)]
enum Message {
    OpenPath,
    PathOpened(Option<PathBuf>),
    DataViewMsg(data_view::Message),
    ToggleDebug,
    RefreshDebug,
    DebugAction(text_editor::Action),
}

impl ClausewitzViewer {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                view: View::Default,
                debug_panel: debug_panel::DebugPanel::new(),
                debug_visible: false,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenPath => Task::perform(open_path(), Message::PathOpened),
            Message::PathOpened(result) => {
                if let Some(path) = result {
                    log::info!("Opening game folder {}", path.display());
                    let (view, task) = data_view::DataView::new(path);
                    self.view = View::DataView(Box::new(view));
                    task.map(Message::DataViewMsg)
                } else {
                    Task::none()
                }
            }
            Message::DataViewMsg(message) => {
                if let View::DataView(view) = &mut self.view {
                    return view.update(message).map(Message::DataViewMsg);
                }

                Task::none()
            }
            Message::ToggleDebug => {
                self.debug_visible = !self.debug_visible;
                if self.debug_visible {
                    self.debug_panel.refresh();
                }
                Task::none()
            }
            Message::RefreshDebug => {
                if self.debug_visible {
                    self.debug_panel.refresh();
                }
                Task::none()
            }
            Message::DebugAction(action) => {
                self.debug_panel.interact(action);
                Task::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let keyboard = iced::event::listen_with(|event, _status, _window| match event {
            iced::Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modifiers,
                repeat,
                ..
            }) => {
                if is_debug_shortcut(&key, modifiers, repeat) {
                    Some(Message::ToggleDebug)
                } else if key.as_ref() == keyboard::Key::Character("o") && modifiers.command() {
                    Some(Message::OpenPath)
                } else {
                    None
                }
            }
            _ => None,
        });
        let debug = if self.debug_visible {
            iced::time::every(Duration::from_millis(250)).map(|_| Message::RefreshDebug)
        } else {
            Subscription::none()
        };
        let loading = match &self.view {
            View::DataView(view) => view.subscription().map(Message::DataViewMsg),
            View::Default => Subscription::none(),
        };
        Subscription::batch([keyboard, debug, loading])
    }

    fn view(&self) -> Element<'_, Message> {
        let controls = row![button("Open Clausewitz Game Path...").on_press(Message::OpenPath)];

        let main: Element<'_, Message> = match &self.view {
            View::Default => container(controls)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
            View::DataView(view) => column![
                container(controls).padding(10),
                container(view.view().map(Message::DataViewMsg)).height(Length::Fill),
            ]
            .height(Length::Fill)
            .into(),
        };

        if self.debug_visible {
            column![
                container(main).width(Length::Fill).height(Length::Fill),
                self.debug_panel.view(Message::DebugAction),
            ]
            .height(Length::Fill)
            .into()
        } else {
            main
        }
    }
}

fn is_debug_shortcut(key: &keyboard::Key, modifiers: keyboard::Modifiers, repeat: bool) -> bool {
    key.as_ref() == keyboard::Key::Character("`")
        && modifiers.control()
        && !modifiers.shift()
        && !modifiers.alt()
        && !modifiers.logo()
        && !repeat
}

async fn open_path() -> Option<PathBuf> {
    let picked_file = rfd::AsyncFileDialog::new()
        .set_title("Choose folder...")
        .pick_folder()
        .await?;

    Some(picked_file.path().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_shortcut_requires_control_backquote_without_repeat() {
        let key = keyboard::Key::Character("`".into());
        assert!(is_debug_shortcut(&key, keyboard::Modifiers::CTRL, false));
        assert!(!is_debug_shortcut(
            &key,
            keyboard::Modifiers::empty(),
            false
        ));
        assert!(!is_debug_shortcut(&key, keyboard::Modifiers::CTRL, true));
        assert!(!is_debug_shortcut(
            &key,
            keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT,
            false
        ));
        assert!(!is_debug_shortcut(
            &keyboard::Key::Character("o".into()),
            keyboard::Modifiers::CTRL,
            false
        ));
    }

    #[test]
    fn toggling_panel_does_not_change_the_current_view() {
        let (mut viewer, _) = ClausewitzViewer::new();
        let _ = viewer.update(Message::ToggleDebug);
        assert!(viewer.debug_visible);
        assert!(matches!(viewer.view, View::Default));
        let _ = viewer.update(Message::ToggleDebug);
        assert!(!viewer.debug_visible);
        assert!(matches!(viewer.view, View::Default));
    }
}
