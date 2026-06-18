#[tauri::command]
pub fn mpv_play(path: String) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("媒体路径不能为空".to_string());
    }

    Err("mpv 兜底播放尚未实现，后续会接入进程启动和控制".to_string())
}
