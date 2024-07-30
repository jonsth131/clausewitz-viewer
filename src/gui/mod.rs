use iced::executor;
use iced::keyboard;
use iced::theme::Theme;
use iced::widget::{button, container, row};
use iced::{Application, Command, Element, Font, Length, Settings, Subscription};

use std::path::PathBuf;

use crate::game::check_game;
use crate::game::Game;

mod hoi4_view;
mod stellaris_view;

pub fn run() -> iced::Result {
    ClausewitzViewer::run(Settings {
        default_font: Font::MONOSPACE,
        ..Settings::default()
    })
}

#[derive(Debug, Default)]
enum View {
    #[default]
    Default,
    Hoi4(hoi4_view::Hoi4View),
    Stellaris(stellaris_view::StellarisView),
}

#[derive(Default)]
struct ClausewitzViewer {
    view: View,
    file: Option<PathBuf>,
}

#[derive(Debug, Clone)]
enum Message {
    OpenPath,
    PathOpened(Result<PathBuf, Error>),
    GamePathOpened(Game),
    Hoi4Message(hoi4_view::Message),
    StellarisMessage(stellaris_view::Message),
}

impl Application for ClausewitzViewer {
    type Message = Message;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        (
            Self {
                view: View::Default,
                file: None,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Clausewitz Viewer")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::OpenPath => Command::perform(open_path(), Message::PathOpened),
            Message::PathOpened(result) => {
                if let Ok(path) = result {
                    self.file = Some(path.clone());
                    return Command::perform(check_game(path.clone()), Message::GamePathOpened);
                }

                Command::none()
            }
            Message::GamePathOpened(game) => match game {
                Game::Hoi4 => {
                    let (view, task) =
                        hoi4_view::Hoi4View::new(self.file.as_ref().unwrap().clone());
                    self.view = View::Hoi4(view);

                    task.map(Message::Hoi4Message)
                }
                Game::Stellaris => {
                    let (view, task) =
                        stellaris_view::StellarisView::new(self.file.as_ref().unwrap().clone());
                    self.view = View::Stellaris(view);

                    task.map(Message::StellarisMessage)
                }
                _ => Command::none(),
            },
            Message::Hoi4Message(message) => {
                if let View::Hoi4(view) = &mut self.view {
                    let _ = view.update(message);
                }

                Command::none()
            }
            Message::StellarisMessage(message) => {
                if let View::Stellaris(view) = &mut self.view {
                    let _ = view.update(message);
                }

                Command::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        keyboard::on_key_press(|key, modifiers| match key.as_ref() {
            keyboard::Key::Character("o") if modifiers.command() => Some(Message::OpenPath),
            _ => None,
        })
    }

    fn view(&self) -> Element<Message> {
        let controls = row![button("Open Clausewitz Game Path...").on_press(Message::OpenPath)];

        match &self.view {
            View::Default => {
                return container(controls)
                    .center_x()
                    .center_y()
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            }
            View::Hoi4(view) => return view.view().map(|m| Message::Hoi4Message(m)),
            View::Stellaris(view) => return view.view().map(|m| Message::StellarisMessage(m)),
        }
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

#[derive(Debug, Clone)]
pub enum Error {
    DialogClosed,
}

async fn open_path() -> Result<PathBuf, Error> {
    let picked_file = rfd::AsyncFileDialog::new()
        .set_title("Choose folder...")
        .pick_folder()
        .await
        .ok_or(Error::DialogClosed)?;

    Ok(picked_file.path().to_owned())
}
