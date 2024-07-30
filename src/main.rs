mod file;
mod game;
mod gui;
mod hoi4;
mod parser;
mod stellaris;

pub fn main() -> iced::Result {
    gui::run()
}
