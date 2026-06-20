use std::{path::PathBuf, process::Command};

use crate::media_server;

#[tauri::command]
pub fn mpv_play(path: String) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("媒体路径不能为空".to_string());
    }

    let path = PathBuf::from(path.trim().trim_matches('"'));
    let playback_path = media_server::prepare_external_playback_path(&path)?;

    match Command::new("mpv").arg(&playback_path).spawn() {
        Ok(_) => Ok(()),
        Err(mpv_error) => open_with_system_player(&playback_path).map_err(|system_error| {
            format!("启动 mpv 失败：{mpv_error}；系统默认播放器也启动失败：{system_error}")
        }),
    }
}

#[cfg(target_os = "windows")]
fn open_with_system_player(path: &PathBuf) -> Result<(), String> {
    Command::new("cmd")
        .args(["/C", "start", "", &path.to_string_lossy()])
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[cfg(target_os = "macos")]
fn open_with_system_player(path: &PathBuf) -> Result<(), String> {
    Command::new("open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_with_system_player(path: &PathBuf) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}
