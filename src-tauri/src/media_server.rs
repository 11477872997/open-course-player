#[tauri::command]
pub fn get_media_url(path: String) -> Result<String, String> {
    if path.trim().is_empty() {
        return Err("媒体路径不能为空".to_string());
    }

    Err("本地 HTTP 媒体服务尚未实现，后续会提供支持分段请求的 127.0.0.1 地址".to_string())
}
