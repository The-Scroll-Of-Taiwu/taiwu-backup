#![windows_subsystem = "windows"]

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use log::{debug, error};
use simplelog::{Config, LevelFilter, WriteLogger};
use tray_item::TrayItem;

use taiwu::Taiwu;

const LOG_TEMP_FOLDER_NAME: &'static str = "TaiwuBackupLogs";

use std::process::Command;

fn main() {
    let log_folder = temp_log_folder();
    let log_file = temp_log_file(&log_folder).unwrap();
    let _ = WriteLogger::init(LevelFilter::Info, Config::default(), log_file);

    let mut tray = TrayItem::new("Taiwu Backup", "TAIWU_ICON_1").unwrap();

    let tw = match Taiwu::new() {
        Ok(tw) => tw,
        Err(e) => {
            error!("[new] error: {:?}", e);
            return;
        }
    };

    debug!("{:?}", tw);

    let tw = Arc::new(tw);

    tray.add_label("[*正在运行中]").unwrap();

    let game_folder = tw.game_root();
    tray.add_menu_item("打开游戏目录", move || {
        debug!("Open game folder occurred!");
        open_folder_in_explorer(&game_folder);
    })
    .unwrap();

    let backup_folder = tw.backup_root();
    tray.add_menu_item("打开备份目录", move || {
        debug!("Open backup folder occurred!");
        open_folder_in_explorer(&backup_folder);
    })
    .unwrap();

    tray.add_menu_item("打开日志目录", move || {
        debug!("Open log folder occurred!");
        open_folder_in_explorer(&log_folder);
    })
    .unwrap();

    let tw1 = Arc::clone(&tw);
    tray.add_menu_item("退出", move || {
        debug!("Quit occurred!");
        tw1.unwatch();
    })
    .unwrap();

    if let Err(e) = tw.backup_once() {
        error!("[backup_once] error: {:?}", e);
        return;
    }

    let handle = thread::spawn(move || {
        if let Err(e) = tw.watch() {
            error!("[watch] error: {:?}", e);
            return;
        }
    });

    handle.join().unwrap();
}

fn temp_log_file(folder: &Path) -> io::Result<fs::File> {
    fs::create_dir_all(folder)?;

    let now = chrono::offset::Local::now();
    let timestamp = now.timestamp_nanos();
    let name = format!("{}.log", timestamp);

    let file_path = folder.join(&name);

    fs::File::create(file_path)
}

fn temp_log_folder() -> PathBuf {
    let temp = std::env::temp_dir();
    temp.join(LOG_TEMP_FOLDER_NAME)
}

fn open_folder_in_explorer(folder: &Path) {
    Command::new("explorer")
        .arg(folder)
        .spawn()
        .unwrap();
}