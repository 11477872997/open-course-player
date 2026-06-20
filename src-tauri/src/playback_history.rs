use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackHistory {
    roots: Vec<String>,
    active_root_path: Option<String>,
    last_media_path: Option<String>,
    updated_at: Option<u64>,
}

#[tauri::command]
pub fn load_playback_history(app: AppHandle) -> Result<PlaybackHistory, String> {
    let path = history_path(&app)?;
    if !path.exists() {
        return Ok(PlaybackHistory::default());
    }

    let content =
        fs::read_to_string(&path).map_err(|error| format!("读取历史记录失败：{error}"))?;
    serde_json::from_str(&content).map_err(|error| format!("解析历史记录失败：{error}"))
}

#[tauri::command]
pub fn save_playback_history(app: AppHandle, mut history: PlaybackHistory) -> Result<(), String> {
    history.roots = normalize_roots(history.roots);
    history.updated_at = Some(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default(),
    );

    let path = history_path(&app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("创建历史记录目录失败：{error}"))?;
    }

    let content = serde_json::to_string_pretty(&history)
        .map_err(|error| format!("序列化历史记录失败：{error}"))?;
    fs::write(path, content).map_err(|error| format!("保存历史记录失败：{error}"))
}

fn history_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法获取应用数据目录：{error}"))?;
    Ok(dir.join("playback-history.json"))
}

fn normalize_roots(roots: Vec<String>) -> Vec<String> {
    let mut result = Vec::new();
    for root in roots {
        if root.trim().is_empty() || result.iter().any(|item| item == &root) {
            continue;
        }
        result.push(root);
    }
    result
}
