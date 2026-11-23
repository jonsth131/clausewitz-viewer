mod file;
mod game;
mod gui;
mod logging;
mod parser;

pub fn main() -> iced::Result {
    logging::init().expect("failed to initialize application logging");
    log::info!("Starting Clausewitz Viewer");
    gui::run()
}
