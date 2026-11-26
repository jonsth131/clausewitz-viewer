mod common;
mod file;
mod game;
mod gui;
mod hoi4;
mod logging;
mod parser;
mod stellaris;

pub fn main() -> iced::Result {
    logging::init().expect("failed to initialize application logging");
    log::info!("Starting Clausewitz Viewer");
    gui::run()
}
