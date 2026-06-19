use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[tauri::command]
pub fn reveal_path(path: String) -> Result<(), String> {
    let path = normalize_path(path.trim());
    if path.is_empty() {
        return Err("路径不能为空".to_string());
    }

    let target = PathBuf::from(path);
    if !target.exists() {
        return Err(format!("路径不存在：{}", target.display()));
    }

    reveal_in_file_manager(&target)
}

fn normalize_path(path: &str) -> String {
    let path = path.trim_matches('"');
    let path = path
        .strip_prefix("file:///")
        .or_else(|| path.strip_prefix("file://"))
        .unwrap_or(path);

    if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else {
        path.strip_prefix(r"\\?\").unwrap_or(path).to_string()
    }
}

#[cfg(target_os = "windows")]
fn reveal_in_file_manager(path: &Path) -> Result<(), String> {
    let mut command = Command::new("explorer.exe");

    if path.is_file() {
        command.arg("/select,").arg(path);
    } else {
        command.arg(path);
    }

    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("打开文件所在位置失败：{error}"))
}

#[cfg(target_os = "macos")]
fn reveal_in_file_manager(path: &Path) -> Result<(), String> {
    let mut command = Command::new("open");
    if path.is_file() {
        command.arg("-R").arg(path);
    } else {
        command.arg(path);
    }

    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("打开文件所在位置失败：{error}"))
}

#[cfg(target_os = "linux")]
fn reveal_in_file_manager(path: &Path) -> Result<(), String> {
    let target = if path.is_file() {
        path.parent().unwrap_or(path)
    } else {
        path
    };

    Command::new("xdg-open")
        .arg(target)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("打开文件所在位置失败：{error}"))
}

#[cfg(any(target_os = "ios", target_os = "android"))]
fn reveal_in_file_manager(_path: &Path) -> Result<(), String> {
    Err("移动端不支持打开系统文件所在位置".to_string())
}

#[cfg(all(
    unix,
    not(target_os = "macos"),
    not(target_os = "linux"),
    not(target_os = "ios"),
    not(target_os = "android")
))]
fn reveal_in_file_manager(_path: &Path) -> Result<(), String> {
    Err("当前平台暂不支持打开文件所在位置".to_string())
}
