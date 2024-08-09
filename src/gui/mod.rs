use iced::executor;
use iced::keyboard;
use iced::theme::Theme;
use iced::widget::{button, container, row};
use iced::{Application, Command, Element, Font, Length, Settings, Subscription};

use std::path::PathBuf;

mod data_view;

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
    DataView(data_view::DataView),
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
    DataViewMessage(data_view::Message),
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
                }

                let (view, task) = data_view::DataView::new(self.file.as_ref().unwrap().clone());
                self.view = View::DataView(view);

                task.map(Message::DataViewMessage)
            }
            Message::DataViewMessage(message) => {
                if let View::DataView(view) = &mut self.view {
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
            View::DataView(view) => return view.view().map(|m| Message::DataViewMessage(m)),
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
