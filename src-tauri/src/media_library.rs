use serde::Serialize;
use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaTreeNode {
    id: String,
    name: String,
    path: String,
    kind: String,
    playable: bool,
    engine: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<MediaTreeNode>>,
}

#[tauri::command]
pub fn scan_media_root(root_path: String) -> Result<Vec<MediaTreeNode>, String> {
    let root = PathBuf::from(root_path);
    let root = root
        .canonicalize()
        .map_err(|e| format!("无法读取目录：{e}"))?;

    if !root.is_dir() {
        return Err("请选择一个有效目录".to_string());
    }

    scan_children(&root, &root, 0)
}

fn scan_children(root: &Path, dir: &Path, depth: usize) -> Result<Vec<MediaTreeNode>, String> {
    if depth > 8 {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(dir)
        .map_err(|e| format!("无法扫描目录 {}：{e}", dir.display()))?
        .filter_map(Result::ok)
        .filter(|entry| !is_hidden_name(&entry.file_name().to_string_lossy()))
        .collect::<Vec<_>>();

    entries.sort_by_key(|entry| {
        let is_file = entry.file_type().map(|ty| ty.is_file()).unwrap_or(false);
        (is_file, entry.file_name().to_string_lossy().to_lowercase())
    });

    let mut nodes = Vec::new();

    for entry in entries {
        let path = entry.path();
        let safe_path = path
            .canonicalize()
            .map_err(|e| format!("无法读取路径 {}：{e}", path.display()))?;

        if !safe_path.starts_with(root) {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        let file_type = entry
            .file_type()
            .map_err(|e| format!("无法读取文件类型 {}：{e}", path.display()))?;

        if file_type.is_dir() {
            let children = scan_children(root, &safe_path, depth + 1)?;
            if children.is_empty() {
                continue;
            }

            nodes.push(MediaTreeNode {
                id: stable_id(&safe_path),
                name,
                path: safe_path.to_string_lossy().to_string(),
                kind: "folder".to_string(),
                playable: false,
                engine: "unsupported".to_string(),
                children: Some(children),
            });
            continue;
        }

        if !file_type.is_file() {
            continue;
        }

        let classification = classify_file(&name);
        if classification.kind == "unknown" {
            continue;
        }

        nodes.push(MediaTreeNode {
            id: stable_id(&safe_path),
            name,
            path: safe_path.to_string_lossy().to_string(),
            kind: classification.kind.to_string(),
            playable: classification.playable,
            engine: classification.engine.to_string(),
            children: None,
        });
    }

    Ok(nodes)
}

struct Classification {
    kind: &'static str,
    playable: bool,
    engine: &'static str,
}

fn classify_file(name: &str) -> Classification {
    let ext = Path::new(name)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{}", value.to_lowercase()))
        .unwrap_or_default();

    match ext.as_str() {
        ".ts" | ".m2ts" | ".mts" => Classification {
            kind: "video",
            playable: true,
            engine: "mpegts",
        },
        ".m3u8" => Classification {
            kind: "video",
            playable: true,
            engine: "easy-player",
        },
        ".mp4" | ".m4v" | ".webm" => Classification {
            kind: "video",
            playable: true,
            engine: "web-video",
        },
        ".mp3" | ".wav" | ".ogg" | ".flac" => Classification {
            kind: "audio",
            playable: true,
            engine: "web-video",
        },
        ".mkv" | ".avi" | ".flv" | ".mov" | ".wmv" | ".rmvb" | ".vob" => Classification {
            kind: "video",
            playable: true,
            engine: "mpv",
        },
        ".srt" | ".ass" | ".vtt" => Classification {
            kind: "subtitle",
            playable: false,
            engine: "unsupported",
        },
        _ => Classification {
            kind: "unknown",
            playable: false,
            engine: "unsupported",
        },
    }
}

fn is_hidden_name(name: &str) -> bool {
    name.starts_with('.') || name.eq_ignore_ascii_case("System Volume Information")
}

fn stable_id(path: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
