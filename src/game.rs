use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Game {
    None,
    Hoi4,
}

pub async fn check_game(path: PathBuf) -> Game {
    if PathBuf::from(path.clone().into_os_string().into_string().unwrap() + "/hoi4_rev.txt")
        .exists()
    {
        Game::Hoi4
    } else {
        Game::None
    }
}
