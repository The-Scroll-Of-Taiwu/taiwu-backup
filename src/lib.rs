use std::path::{Path, PathBuf};
use std::io;
use std::fs;
use std::sync::Mutex;

use log::{trace, debug, info, warn, error};
use thiserror::Error;
use notify::{event, RecommendedWatcher, RecursiveMode, Watcher, Config, Event};
use directories::BaseDirs;

mod game_root;

use game_root::GameRoot;

pub type Result<T> = std::result::Result<T, TaiwuError>;

const APPDATA_FOLDER_NAME: &'static str = "TaiwuBackup";
const BACKUP_FOLDER_NAME: &'static str = "BackupData";
const TAIWU_GAME_SAVE_ROOT_NAME: &'static str = "Save";
const TAIWU_GAME_SAVE_FILE_NAME: &'static str = "local.sav";
const TAIWU_GAME_SAVE_WORLD_NUMBER_MAX: usize = 5;

#[derive(Debug)]
pub struct Taiwu {
    game_root: PathBuf,
    backup_root: PathBuf,
    watcher: Mutex<Option<RecommendedWatcher>>,
}


#[derive(Error, Debug)]
pub enum TaiwuError {
    #[error("game root path not found")]
    GameRootNotFound,
    #[error("defatul backup destination path not available")]
    BackupRootDefaultNotAvailable,
    #[error("IO error")]
    IoError(#[from] io::Error),
    #[error("notify error")]
    NotifyError(#[from] notify::Error),
    #[error("unknown error")]
    Unknown,
}

impl Taiwu {
    pub fn new() -> Result<Taiwu> {
        if let Some(root) = GameRoot::auto() {
            let game_root = root.path().to_owned();
            let backup_root = get_backup_root_default()?;
            let watcher = Mutex::new(None);
            Ok(Taiwu { game_root, backup_root, watcher })
        } else {
            Err(TaiwuError::GameRootNotFound)
        }
    }

    pub fn with_path(path: impl AsRef<Path>) -> Result<Taiwu> {
        if let Some(root) = GameRoot::new(path) {
            let game_root = root.path().to_owned();
            let backup_root = get_backup_root_default()?;
            let watcher = Mutex::new(None);
            Ok(Taiwu { game_root, backup_root, watcher })
        } else {
            Err(TaiwuError::GameRootNotFound)
        }
    }

    pub fn game_root(&self) -> PathBuf {
        self.game_root.clone()
    }

    pub fn backup_root(&self) -> PathBuf {
        self.backup_root.clone()
    }

    fn save_root(&self) -> PathBuf {
        self.game_root.join(TAIWU_GAME_SAVE_ROOT_NAME)
    }

    fn save_file(&self, world: usize) -> PathBuf {
        self.save_root().join(format!("world_{}", world)).join(TAIWU_GAME_SAVE_FILE_NAME)
    }

    pub fn backup_once(&self) -> Result<()> {
        trace!("do backup once");
        for world in 1..=TAIWU_GAME_SAVE_WORLD_NUMBER_MAX {
            let save = self.save_file(world);
            if save.is_file() {
                self.backup(&save)?;
            }
        }
        Ok(())
    }

    pub fn watch(&self) -> Result<()> {
        let (tx, rx) = std::sync::mpsc::channel();
    
        // Automatically select the best implementation for your platform.
        // You can also access each implementation directly e.g. INotifyWatcher.
        let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
    
        debug!("RecommendedWatcher::kind() is {:?}", RecommendedWatcher::kind());

        let watched = self.save_root();
    
        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        watcher.watch(&watched, RecursiveMode::Recursive)?;

        info!("Watching `{}`", watched.display());
        info!("Then will backup to `{}`", watched.display());

        *self.watcher.lock().unwrap() = Some(watcher);

        for res in rx {
            match res {
                Ok(event) => {
                    debug!("{}", print_event(&event));
                    self.process(event)?;
                },
                Err(e) => error!("watch error: {:?}", e),
            }
        }

        info!("End watching");

        Ok(())
    }

    pub fn unwatch(&self) {
        if let Some(watcher) = self.watcher.lock().unwrap().take() {
            drop(watcher);
            trace!("drop the member Taiwu::watcher");
        }
    }

    fn process(&self, event: Event) -> io::Result<()> {
        for path in &event.paths {
            if !self.is_save_file(path) {
                continue;
            }
            match event.kind {
                event::EventKind::Modify(ref modify_kind) => {
                    match modify_kind {
                        event::ModifyKind::Any => {
                            trace!("file changed, backup it");
                            self.backup(path)?;
                        },
                        event::ModifyKind::Name(event::RenameMode::From) => {
                            trace!("rename to other file, do nothing");
                        }
                        _ => warn!("unexpected modify type (not ModifyKind::Any), do nothing"),
                    }
                }
                _ => trace!("not modify event, do nothing"),
            };
        }

        Ok(())
    }

    fn is_save_file(&self, path: &Path) -> bool {
        for world in 1..=TAIWU_GAME_SAVE_WORLD_NUMBER_MAX {
            if path == self.save_file(world) {
                return true;
            }
        }
        false
    }

    fn backup(&self, src: &Path) -> io::Result<()> {
        let file_name = new_backup_file_name_now();
        let folder_name = src.parent().unwrap().file_name().unwrap();
        let dst = self.backup_root.join(folder_name).join(file_name);
        debug!("[now do it] backup `{}` to `{}...`", src.display(), dst.display());

        fs::create_dir_all(dst.parent().unwrap())?;
        fs::copy(src, dst.clone())?;

        info!("[Backup] {}", src.display());
        info!("[    to] {}", dst.display());

        Ok(())
    }
}

fn get_backup_root_default() -> Result<PathBuf> {
    if let Some(base_dirs) = BaseDirs::new() {
        let backup_root = base_dirs.data_local_dir().to_path_buf().join(APPDATA_FOLDER_NAME).join(BACKUP_FOLDER_NAME);
        Ok(backup_root)
    } else {
        Err(TaiwuError::BackupRootDefaultNotAvailable)
    }
}

fn print_event(event: &Event) -> String {
    let paths = &event.paths;
    let path_info = if paths.len() == 1 {
        paths.get(0).unwrap().display().to_string()
    } else {
        format!("{:?}", paths)
    };
    format!("[{:?}] `{}`", event.kind, path_info)
}

fn new_backup_file_name_now() -> String {
    let now = chrono::offset::Local::now();
    let timestamp = now.timestamp_nanos();
    format!("{}.{}", TAIWU_GAME_SAVE_FILE_NAME, timestamp)
}