use serde::Serialize;
use std::{
    collections::hash_map::DefaultHasher,
    fs::{self, File},
    hash::{Hash, Hasher},
    io::Read,
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
                path: display_path(&safe_path),
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

        let classification = classify_file(&safe_path, &name);
        if classification.kind == "unknown" {
            continue;
        }

        nodes.push(MediaTreeNode {
            id: stable_id(&safe_path),
            name,
            path: display_path(&safe_path),
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

fn classify_file(path: &Path, name: &str) -> Classification {
    let ext = Path::new(name)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{}", value.to_lowercase()))
        .unwrap_or_default();

    if is_document_extension(&ext) {
        return Classification {
            kind: "document",
            playable: true,
            engine: "document",
        };
    }

    if let Some(format) = sniff_file_format(path) {
        return classify_format(format);
    }

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
        ".mp4" | ".m4v" | ".webm" | ".ogv" | ".sz" => Classification {
            kind: "video",
            playable: true,
            engine: "web-video",
        },
        ".mp3" | ".wav" | ".ogg" | ".flac" | ".m4a" | ".aac" | ".opus" | ".wma" => Classification {
            kind: "audio",
            playable: true,
            engine: "web-video",
        },
        ".mkv" | ".avi" | ".flv" | ".mov" | ".wmv" | ".rmvb" | ".vob" | ".3gp" | ".mpeg"
        | ".mpg" => Classification {
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

fn is_document_extension(ext: &str) -> bool {
    matches!(
        ext,
        ".pdf" | ".docx" | ".doc" | ".xlsx" | ".xls" | ".pptx" | ".ppt"
    )
}

#[derive(Clone, Copy)]
enum SniffedFormat {
    MpegTs,
    Mp4,
    Mp3,
    Wav,
    Flac,
    Ogg,
    Webm,
}

fn classify_format(format: SniffedFormat) -> Classification {
    match format {
        SniffedFormat::MpegTs => Classification {
            kind: "video",
            playable: true,
            engine: "mpegts",
        },
        SniffedFormat::Mp4 | SniffedFormat::Webm => Classification {
            kind: "video",
            playable: true,
            engine: "web-video",
        },
        SniffedFormat::Mp3 | SniffedFormat::Wav | SniffedFormat::Flac | SniffedFormat::Ogg => {
            Classification {
                kind: "audio",
                playable: true,
                engine: "web-video",
            }
        }
    }
}

fn sniff_file_format(path: &Path) -> Option<SniffedFormat> {
    let mut file = File::open(path).ok()?;
    let mut buffer = [0u8; 64 * 1024];
    let read = file.read(&mut buffer).ok()?;
    let data = &buffer[..read];

    if data.get(0..5) == Some(b"%PDF-") {
        return None;
    }

    if looks_like_mpeg_ts(data) {
        return Some(SniffedFormat::MpegTs);
    }

    if data.len() >= 12 && data.get(4..8) == Some(b"ftyp") {
        return Some(SniffedFormat::Mp4);
    }

    if data.get(0..4) == Some(b"\x1a\x45\xdf\xa3") {
        return Some(SniffedFormat::Webm);
    }

    if data.get(0..3) == Some(b"ID3") || looks_like_mp3_frame(data) {
        return Some(SniffedFormat::Mp3);
    }

    if data.get(0..4) == Some(b"RIFF") && data.get(8..12) == Some(b"WAVE") {
        return Some(SniffedFormat::Wav);
    }

    if data.get(0..4) == Some(b"fLaC") {
        return Some(SniffedFormat::Flac);
    }

    if data.get(0..4) == Some(b"OggS") {
        return Some(SniffedFormat::Ogg);
    }

    None
}

fn looks_like_mpeg_ts(data: &[u8]) -> bool {
    [188usize, 192, 204].iter().copied().any(|packet_size| {
        let search_len = data.len().min(packet_size * 8);
        (0..packet_size.min(search_len)).any(|offset| {
            let possible = ((data.len().saturating_sub(offset)) / packet_size).min(5);
            possible >= 3
                && (0..possible).all(|nth| {
                    let index = offset + nth * packet_size;
                    index < data.len() && data[index] == 0x47
                })
        })
    })
}

fn looks_like_mp3_frame(data: &[u8]) -> bool {
    data.windows(2)
        .take(1024)
        .any(|bytes| bytes[0] == 0xff && bytes[1] & 0xe0 == 0xe0)
}

fn is_hidden_name(name: &str) -> bool {
    name.starts_with('.') || name.eq_ignore_ascii_case("System Volume Information")
}

fn stable_id(path: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn display_path(path: &Path) -> String {
    let value = path.to_string_lossy().to_string();
    value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
}
