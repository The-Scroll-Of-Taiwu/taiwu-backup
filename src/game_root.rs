use std::path::{PathBuf, Path};

use log::{debug, error};

const TAIWU_GAME_STEAM_APPID: usize = 838350;

#[derive(Debug)]
pub struct GameRoot {
    path: PathBuf,
}

impl GameRoot {
    pub fn new(path: impl AsRef<Path>) -> Option<GameRoot> {
        let path = path.as_ref();
        if path.is_dir() {
            let path = path.to_owned();
            Some(GameRoot { path })
        } else {
            None
        }
    }

    pub fn auto() -> Option<GameRoot> {
        if let Some(path) = get_game_root_by_appid(TAIWU_GAME_STEAM_APPID) {
            Some(GameRoot { path })
        } else {
            None
        }
    }

    pub fn path(&self) -> &Path {
        self.path.as_ref()
    }
}

fn get_game_root_by_appid(app_id: usize) -> Option<PathBuf> {
    use steamlocate::SteamDir;

    let app_id = &(u32::try_from(app_id).unwrap());

    let mut steamdir = SteamDir::locate().unwrap();
    match steamdir.app(app_id) {
        Some(app) => {
            debug!("{:?}", app);
            Some(app.path.to_owned())
        },
        None => {
            error!("could not locate 太吾绘卷 (The Scroll Of Taiwu) on this computer");
            None
        }
    }
}