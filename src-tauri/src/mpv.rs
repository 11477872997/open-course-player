#[tauri::command]
pub fn mpv_play(path: String) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("媒体路径不能为空".to_string());
    }

    std::process::Command::new("mpv")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("启动 mpv 失败，请确认已安装 mpv 并加入 PATH：{error}"))
}
