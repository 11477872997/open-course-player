use serde::Serialize;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    fs::{self, File},
    hash::{Hash, Hasher},
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

static MEDIA_SERVER: OnceLock<Arc<MediaServer>> = OnceLock::new();

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSourceInfo {
    url: String,
    mime: String,
    size: u64,
    duration: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleTrackInfo {
    label: String,
    path: String,
    format: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleSourceInfo {
    url: String,
    mime: String,
    format: String,
}

#[derive(Clone)]
struct MediaEntry {
    path: PathBuf,
    mime: String,
    size: u64,
}

struct MediaServer {
    address: String,
    entries: Mutex<HashMap<String, MediaEntry>>,
}

#[tauri::command]
pub fn create_media_source(path: String) -> Result<MediaSourceInfo, String> {
    let path = normalize_path(&path);
    let path = PathBuf::from(path)
        .canonicalize()
        .map_err(|error| format!("媒体文件不存在或无法读取：{error}"))?;

    if !path.is_file() {
        return Err("请选择一个媒体文件".to_string());
    }

    let size = path
        .metadata()
        .map_err(|error| format!("无法读取媒体文件信息：{error}"))?
        .len();
    let mime = mime_from_path(&path).to_string();
    let duration = probe_duration(&path);
    let token = media_token(&path, size);
    let server = media_server()?;

    server.entries.lock().map_err(|_| "媒体服务状态异常".to_string())?.insert(
        token.clone(),
        MediaEntry {
            path,
            mime: mime.clone(),
            size,
        },
    );

    Ok(MediaSourceInfo {
        url: format!("http://{}/media/{}", server.address, token),
        mime,
        size,
        duration,
    })
}

#[tauri::command]
pub fn find_subtitle_tracks(path: String) -> Result<Vec<SubtitleTrackInfo>, String> {
    let path = PathBuf::from(normalize_path(&path))
        .canonicalize()
        .map_err(|error| format!("媒体文件不存在或无法读取：{error}"))?;
    let parent = path.parent().ok_or_else(|| "媒体文件没有所在目录".to_string())?;
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "无法识别媒体文件名".to_string())?;

    let mut tracks = Vec::new();
    for format in ["vtt", "srt", "ass"] {
        let subtitle_path = parent.join(format!("{stem}.{format}"));
        if subtitle_path.is_file() {
            tracks.push(SubtitleTrackInfo {
                label: format!("{} 字幕", format.to_uppercase()),
                path: display_path(&subtitle_path),
                format: format.to_string(),
            });
        }
    }

    Ok(tracks)
}

#[tauri::command]
pub fn create_subtitle_source(path: String) -> Result<SubtitleSourceInfo, String> {
    let path = PathBuf::from(normalize_path(&path))
        .canonicalize()
        .map_err(|error| format!("字幕文件不存在或无法读取：{error}"))?;

    if !path.is_file() {
        return Err("请选择一个字幕文件".to_string());
    }

    let format = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();

    let source_path = match format.as_str() {
        "vtt" => path,
        "srt" => write_vtt_temp_file(&path)?,
        "ass" => return Err("ASS 字幕当前不能由内置浏览器播放器直接渲染，可通过 mpv 兜底播放".to_string()),
        _ => return Err("当前字幕格式暂不支持".to_string()),
    };

    let size = source_path
        .metadata()
        .map_err(|error| format!("无法读取字幕文件信息：{error}"))?
        .len();
    let token = media_token(&source_path, size);
    let server = media_server()?;
    server.entries.lock().map_err(|_| "媒体服务状态异常".to_string())?.insert(
        token.clone(),
        MediaEntry {
            path: source_path,
            mime: "text/vtt; charset=utf-8".to_string(),
            size,
        },
    );

    Ok(SubtitleSourceInfo {
        url: format!("http://{}/media/{}", server.address, token),
        mime: "text/vtt; charset=utf-8".to_string(),
        format,
    })
}

fn media_server() -> Result<Arc<MediaServer>, String> {
    if let Some(server) = MEDIA_SERVER.get() {
        return Ok(server.clone());
    }

    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|error| format!("无法启动本地媒体服务：{error}"))?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("无法读取本地媒体服务地址：{error}"))?
        .to_string();
    let server = Arc::new(MediaServer {
        address,
        entries: Mutex::new(HashMap::new()),
    });

    let thread_server = server.clone();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let server = thread_server.clone();
            thread::spawn(move || {
                let _ = handle_request(stream, server);
            });
        }
    });

    let _ = MEDIA_SERVER.set(server.clone());
    Ok(MEDIA_SERVER.get().cloned().unwrap_or(server))
}

fn handle_request(stream: TcpStream, server: Arc<MediaServer>) -> Result<(), String> {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .map_err(|error| format!("读取媒体请求失败：{error}"))?;

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_uppercase();
    let path = parts.next().unwrap_or_default().to_string();
    let mut range_header: Option<String> = None;

    loop {
        let mut line = String::new();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| format!("读取媒体请求头失败：{error}"))?;
        if bytes == 0 || line == "\r\n" || line == "\n" {
            break;
        }

        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("range") {
                range_header = Some(value.trim().to_string());
            }
        }
    }

    let mut stream = reader.into_inner();

    if method == "OPTIONS" {
        write_options_response(&mut stream)?;
        return Ok(());
    }

    if method != "GET" && method != "HEAD" {
        write_error_response(&mut stream, 405, "Method Not Allowed")?;
        return Ok(());
    }

    let Some(token) = path.strip_prefix("/media/") else {
        write_error_response(&mut stream, 404, "Not Found")?;
        return Ok(());
    };

    let entry = {
        let entries = server.entries.lock().map_err(|_| "媒体服务状态异常".to_string())?;
        entries.get(token).cloned()
    };

    let Some(entry) = entry else {
        write_error_response(&mut stream, 404, "Not Found")?;
        return Ok(());
    };

    serve_media(&mut stream, &entry, range_header.as_deref(), method == "HEAD")
}

fn serve_media(
    stream: &mut TcpStream,
    entry: &MediaEntry,
    range_header: Option<&str>,
    head_only: bool,
) -> Result<(), String> {
    let mut file = File::open(&entry.path)
        .map_err(|error| format!("无法打开媒体文件 {}：{error}", entry.path.display()))?;
    let (start, end, partial) = parse_range(range_header, entry.size)?;
    let content_length = end.saturating_sub(start).saturating_add(1);
    let status = if partial {
        "HTTP/1.1 206 Partial Content"
    } else {
        "HTTP/1.1 200 OK"
    };

    let mut headers = format!(
        "{status}\r\n\
         Content-Type: {}\r\n\
         Content-Length: {content_length}\r\n\
         Accept-Ranges: bytes\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Headers: Range\r\n\
         Access-Control-Expose-Headers: Content-Length, Content-Range, Accept-Ranges\r\n",
        entry.mime
    );

    if partial {
        headers.push_str(&format!(
            "Content-Range: bytes {start}-{end}/{}\r\n",
            entry.size
        ));
    }

    headers.push_str("Connection: close\r\n\r\n");
    stream
        .write_all(headers.as_bytes())
        .map_err(|error| format!("写入媒体响应头失败：{error}"))?;

    if head_only {
        return Ok(());
    }

    file.seek(SeekFrom::Start(start))
        .map_err(|error| format!("定位媒体文件失败：{error}"))?;

    let mut remaining = content_length;
    let mut buffer = [0u8; 64 * 1024];
    while remaining > 0 {
        let max_read = remaining.min(buffer.len() as u64) as usize;
        let read = file
            .read(&mut buffer[..max_read])
            .map_err(|error| format!("读取媒体文件失败：{error}"))?;
        if read == 0 {
            break;
        }
        stream
            .write_all(&buffer[..read])
            .map_err(|error| format!("发送媒体数据失败：{error}"))?;
        remaining = remaining.saturating_sub(read as u64);
    }

    Ok(())
}

fn parse_range(range_header: Option<&str>, size: u64) -> Result<(u64, u64, bool), String> {
    if size == 0 {
        return Ok((0, 0, false));
    }

    let Some(range) = range_header else {
        return Ok((0, size - 1, false));
    };
    let Some(range) = range.strip_prefix("bytes=") else {
        return Ok((0, size - 1, false));
    };

    let Some((start_text, end_text)) = range.split_once('-') else {
        return Ok((0, size - 1, false));
    };

    if start_text.is_empty() {
        let suffix = end_text.parse::<u64>().unwrap_or(0).min(size);
        let start = size.saturating_sub(suffix);
        return Ok((start, size - 1, true));
    }

    let start = start_text.parse::<u64>().unwrap_or(0).min(size - 1);
    let end = if end_text.is_empty() {
        size - 1
    } else {
        end_text.parse::<u64>().unwrap_or(size - 1).min(size - 1)
    };

    Ok((start, end.max(start), true))
}

fn write_options_response(stream: &mut TcpStream) -> Result<(), String> {
    stream
        .write_all(
            b"HTTP/1.1 204 No Content\r\n\
              Access-Control-Allow-Origin: *\r\n\
              Access-Control-Allow-Methods: GET, HEAD, OPTIONS\r\n\
              Access-Control-Allow-Headers: Range\r\n\
              Access-Control-Max-Age: 86400\r\n\
              Connection: close\r\n\r\n",
        )
        .map_err(|error| format!("写入媒体响应失败：{error}"))
}

fn write_error_response(stream: &mut TcpStream, code: u16, text: &str) -> Result<(), String> {
    let body = text.as_bytes();
    let response = format!(
        "HTTP/1.1 {code} {text}\r\n\
         Content-Type: text/plain; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .and_then(|_| stream.write_all(body))
        .map_err(|error| format!("写入媒体错误响应失败：{error}"))
}

fn media_token(path: &Path, size: u64) -> String {
    let mut hasher = DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    size.hash(&mut hasher);
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
        .hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn normalize_path(path: &str) -> String {
    let path = path.trim().trim_matches('"');
    if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else {
        path.strip_prefix(r"\\?\").unwrap_or(path).to_string()
    }
}

fn mime_from_path(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "ts" | "m2ts" | "mts" => "video/mp2t",
        "m3u8" => "application/vnd.apple.mpegurl",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "ogv" => "video/ogg",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "flac" => "audio/flac",
        "m4a" | "aac" => "audio/aac",
        "opus" => "audio/opus",
        "wma" => "audio/x-ms-wma",
        "vtt" => "text/vtt; charset=utf-8",
        "srt" => "application/x-subrip; charset=utf-8",
        "ass" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn write_vtt_temp_file(path: &Path) -> Result<PathBuf, String> {
    let bytes = fs::read(path).map_err(|error| format!("无法读取 SRT 字幕：{error}"))?;
    let content = String::from_utf8_lossy(&bytes);
    let vtt = srt_to_vtt(&content);
    let token = media_token(path, bytes.len() as u64);
    let dir = std::env::temp_dir().join("open-course-player-subtitles");
    fs::create_dir_all(&dir).map_err(|error| format!("无法创建字幕缓存目录：{error}"))?;
    let target = dir.join(format!("{token}.vtt"));
    fs::write(&target, vtt).map_err(|error| format!("无法写入字幕缓存：{error}"))?;
    Ok(target)
}

fn srt_to_vtt(content: &str) -> String {
    let mut output = String::from("WEBVTT\n\n");
    for line in content.lines() {
        if line.contains("-->") {
            output.push_str(&line.replace(',', "."));
        } else {
            output.push_str(line);
        }
        output.push('\n');
    }
    output
}

fn display_path(path: &Path) -> String {
    let value = path.to_string_lossy().to_string();
    value
        .strip_prefix(r"\\?\")
        .unwrap_or(&value)
        .to_string()
}

fn probe_duration(path: &Path) -> Option<f64> {
    probe_with_ffprobe(path)
        .or_else(|| probe_ts_duration(path))
        .or_else(|| probe_wav_duration(path))
        .or_else(|| probe_mp3_duration(path))
}

fn probe_with_ffprobe(path: &Path) -> Option<f64> {
    let output = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value > 0.0)
}

fn probe_ts_duration(path: &Path) -> Option<f64> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    if !matches!(ext.as_str(), "ts" | "m2ts" | "mts") {
        return None;
    }

    let data = std::fs::read(path).ok()?;
    let (packet_size, offset) = detect_ts_packet_layout(&data)?;
    let mut first_pts: Option<u64> = None;
    let mut last_pts: Option<u64> = None;
    let mut index = offset;

    while index + packet_size <= data.len() {
        let packet = &data[index..index + packet_size];
        if packet.first().copied() == Some(0x47) {
            if let Some(pts) = parse_ts_packet_pts(packet) {
                if first_pts.is_none() {
                    first_pts = Some(pts);
                }
                last_pts = Some(pts);
            }
        }
        index += packet_size;
    }

    let first = first_pts?;
    let mut last = last_pts?;
    if last < first {
        last += 1u64 << 33;
    }

    let seconds = (last - first) as f64 / 90_000.0;
    (seconds > 0.0).then_some(seconds)
}

fn detect_ts_packet_layout(data: &[u8]) -> Option<(usize, usize)> {
    for packet_size in [188usize, 192, 204] {
        let search_len = data.len().min(packet_size * 8);
        for offset in 0..packet_size.min(search_len) {
            let mut matches = 0;
            let possible = ((data.len().saturating_sub(offset)) / packet_size).min(5);
            for nth in 0..possible {
                let index = offset + nth * packet_size;
                if index < data.len() && data[index] == 0x47 {
                    matches += 1;
                }
            }
            if possible >= 2 && matches == possible {
                return Some((packet_size, offset));
            }
        }
    }

    None
}

fn parse_ts_packet_pts(packet: &[u8]) -> Option<u64> {
    if packet.len() < 19 || packet[0] != 0x47 {
        return None;
    }

    let payload_unit_start = packet[1] & 0x40 != 0;
    if !payload_unit_start {
        return None;
    }

    let adaptation_control = (packet[3] >> 4) & 0x03;
    if adaptation_control == 0 || adaptation_control == 2 {
        return None;
    }

    let mut payload_start = 4usize;
    if adaptation_control == 3 {
        let length = packet.get(4).copied()? as usize;
        payload_start = payload_start.saturating_add(1 + length);
    }

    if payload_start + 14 > packet.len() {
        return None;
    }

    let payload = &packet[payload_start..];
    if payload.get(0..3) != Some(&[0x00, 0x00, 0x01]) {
        return None;
    }

    let pts_dts_flags = (payload[7] >> 6) & 0x03;
    if pts_dts_flags == 0 {
        return None;
    }

    decode_pts(payload.get(9..14)?)
}

fn decode_pts(bytes: &[u8]) -> Option<u64> {
    if bytes.len() != 5 {
        return None;
    }

    Some(
        (((bytes[0] >> 1) as u64 & 0x07) << 30)
            | ((bytes[1] as u64) << 22)
            | (((bytes[2] >> 1) as u64 & 0x7f) << 15)
            | ((bytes[3] as u64) << 7)
            | ((bytes[4] >> 1) as u64 & 0x7f),
    )
}

fn probe_wav_duration(path: &Path) -> Option<f64> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    if ext != "wav" {
        return None;
    }

    let data = std::fs::read(path).ok()?;
    if data.get(0..4) != Some(b"RIFF") || data.get(8..12) != Some(b"WAVE") {
        return None;
    }

    let mut offset = 12usize;
    let mut byte_rate: Option<u32> = None;
    let mut data_size: Option<u32> = None;

    while offset + 8 <= data.len() {
        let chunk_id = &data[offset..offset + 4];
        let chunk_size = u32::from_le_bytes(data[offset + 4..offset + 8].try_into().ok()?);
        let chunk_start = offset + 8;
        let chunk_end = chunk_start.saturating_add(chunk_size as usize).min(data.len());

        if chunk_id == b"fmt " && chunk_start + 12 <= chunk_end {
            byte_rate = Some(u32::from_le_bytes(
                data[chunk_start + 8..chunk_start + 12].try_into().ok()?,
            ));
        } else if chunk_id == b"data" {
            data_size = Some(chunk_size);
        }

        offset = chunk_start + chunk_size as usize + (chunk_size as usize % 2);
    }

    let byte_rate = byte_rate? as f64;
    let data_size = data_size? as f64;
    (byte_rate > 0.0).then_some(data_size / byte_rate)
}

fn probe_mp3_duration(path: &Path) -> Option<f64> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    if ext != "mp3" {
        return None;
    }

    let data = std::fs::read(path).ok()?;
    let mut offset = skip_id3v2(&data);
    let mut seconds = 0.0;
    let mut frames = 0usize;

    while offset + 4 <= data.len() {
        if data[offset] != 0xff || data[offset + 1] & 0xe0 != 0xe0 {
            offset += 1;
            continue;
        }

        let Some((frame_length, frame_seconds)) = parse_mp3_frame(&data[offset..offset + 4]) else {
            offset += 1;
            continue;
        };

        seconds += frame_seconds;
        frames += 1;
        offset = offset.saturating_add(frame_length.max(1));
    }

    (frames > 0 && seconds > 0.0).then_some(seconds)
}

fn skip_id3v2(data: &[u8]) -> usize {
    if data.len() < 10 || data.get(0..3) != Some(b"ID3") {
        return 0;
    }

    let size = ((data[6] as usize & 0x7f) << 21)
        | ((data[7] as usize & 0x7f) << 14)
        | ((data[8] as usize & 0x7f) << 7)
        | (data[9] as usize & 0x7f);
    10 + size
}

fn parse_mp3_frame(header: &[u8]) -> Option<(usize, f64)> {
    if header.len() < 4 {
        return None;
    }

    let version_id = (header[1] >> 3) & 0x03;
    let layer = (header[1] >> 1) & 0x03;
    let bitrate_index = (header[2] >> 4) & 0x0f;
    let sample_rate_index = (header[2] >> 2) & 0x03;
    let padding = ((header[2] >> 1) & 0x01) as usize;

    if version_id == 1 || layer == 0 || bitrate_index == 0 || bitrate_index == 0x0f || sample_rate_index == 3 {
        return None;
    }

    let version = match version_id {
        3 => 1,
        2 => 2,
        0 => 25,
        _ => return None,
    };
    let layer_number = 4 - layer;
    let bitrate = mp3_bitrate(version, layer_number, bitrate_index as usize)? as usize * 1000;
    let sample_rate = mp3_sample_rate(version, sample_rate_index as usize)? as usize;
    let samples_per_frame = if layer_number == 1 {
        384
    } else if layer_number == 3 && version != 1 {
        576
    } else {
        1152
    };

    let frame_length = if layer_number == 1 {
        ((12 * bitrate / sample_rate) + padding) * 4
    } else if layer_number == 3 && version != 1 {
        (72 * bitrate / sample_rate) + padding
    } else {
        (144 * bitrate / sample_rate) + padding
    };

    Some((frame_length, samples_per_frame as f64 / sample_rate as f64))
}

fn mp3_bitrate(version: u8, layer: u8, index: usize) -> Option<u16> {
    const V1_L1: [u16; 16] = [0, 32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448, 0];
    const V1_L2: [u16; 16] = [0, 32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384, 0];
    const V1_L3: [u16; 16] = [0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0];
    const V2_L1: [u16; 16] = [0, 32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256, 0];
    const V2_L2L3: [u16; 16] = [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0];

    let table = match (version == 1, layer) {
        (true, 1) => V1_L1,
        (true, 2) => V1_L2,
        (true, 3) => V1_L3,
        (false, 1) => V2_L1,
        (false, 2 | 3) => V2_L2L3,
        _ => return None,
    };
    table.get(index).copied().filter(|value| *value > 0)
}

fn mp3_sample_rate(version: u8, index: usize) -> Option<u32> {
    let base = [44_100, 48_000, 32_000].get(index).copied()?;
    match version {
        1 => Some(base),
        2 => Some(base / 2),
        25 => Some(base / 4),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn probes_fixture_durations() {
        let root = create_test_media_dir("duration");

        let ts = probe_ts_duration(&root.join("video-ts-timestamp-27s.ts")).unwrap();
        assert!((ts - 27.0).abs() < 0.2, "ts duration was {ts}");

        let wav = probe_wav_duration(&root.join("audio-wav-3s.wav")).unwrap();
        assert!((wav - 3.0).abs() < 0.2, "wav duration was {wav}");

        let mp3 = probe_mp3_duration(&root.join("audio-mp3-3s.mp3")).unwrap();
        assert!((mp3 - 3.0).abs() < 0.2, "mp3 duration was {mp3}");
    }

    #[test]
    fn media_server_serves_byte_ranges() {
        let root = create_test_media_dir("range");
        let source = create_media_source(root.join("audio-wav-3s.wav").to_string_lossy().to_string()).unwrap();
        let url = source.url.strip_prefix("http://").unwrap();
        let (address, path) = url.split_once('/').unwrap();
        let mut stream = TcpStream::connect(address).unwrap();
        let request = format!(
            "GET /{path} HTTP/1.1\r\nHost: {address}\r\nRange: bytes=0-15\r\nConnection: close\r\n\r\n"
        );

        stream.write_all(request.as_bytes()).unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        let response = String::from_utf8_lossy(&response);

        assert!(response.starts_with("HTTP/1.1 206 Partial Content"), "{response}");
        assert!(response.contains("Content-Range: bytes 0-15/"), "{response}");
        assert!(response.contains("Accept-Ranges: bytes"), "{response}");
    }

    fn create_test_media_dir(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "open-course-player-media-server-test-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        write_test_wav(&root.join("audio-wav-3s.wav"), 3);
        write_test_mp3(&root.join("audio-mp3-3s.mp3"), 3);
        write_test_ts(&root.join("video-ts-timestamp-27s.ts"), 27);
        root
    }

    fn write_test_wav(path: &Path, seconds: u32) {
        let sample_rate = 44_100u32;
        let channels = 1u16;
        let bits_per_sample = 16u16;
        let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
        let block_align = channels * bits_per_sample / 8;
        let data_size = sample_rate * channels as u32 * bits_per_sample as u32 / 8 * seconds;
        let mut data = Vec::new();
        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&(36 + data_size).to_le_bytes());
        data.extend_from_slice(b"WAVEfmt ");
        data.extend_from_slice(&16u32.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&channels.to_le_bytes());
        data.extend_from_slice(&sample_rate.to_le_bytes());
        data.extend_from_slice(&byte_rate.to_le_bytes());
        data.extend_from_slice(&block_align.to_le_bytes());
        data.extend_from_slice(&bits_per_sample.to_le_bytes());
        data.extend_from_slice(b"data");
        data.extend_from_slice(&data_size.to_le_bytes());
        data.resize(data.len() + data_size as usize, 0);
        std::fs::write(path, data).unwrap();
    }

    fn write_test_mp3(path: &Path, seconds: u32) {
        let mut data = Vec::new();
        let frame_count = ((seconds as f64 * 44_100.0) / 1152.0).ceil() as usize;
        for _ in 0..frame_count {
            data.extend_from_slice(&[0xff, 0xfb, 0x90, 0x64]);
            data.resize(data.len() + 413, 0);
        }
        std::fs::write(path, data).unwrap();
    }

    fn write_test_ts(path: &Path, seconds: u64) {
        let mut data = Vec::new();
        data.extend_from_slice(&test_ts_packet(0));
        data.extend_from_slice(&test_ts_packet(seconds * 90_000));
        std::fs::write(path, data).unwrap();
    }

    fn test_ts_packet(pts: u64) -> [u8; 188] {
        let mut packet = [0xffu8; 188];
        packet[0] = 0x47;
        packet[1] = 0x41;
        packet[2] = 0x00;
        packet[3] = 0x10;
        let pts = encode_test_pts(pts);
        let payload = [
            0x00, 0x00, 0x01, 0xe0, 0x00, 0x00, 0x80, 0x80, 0x05, pts[0], pts[1], pts[2], pts[3],
            pts[4],
        ];
        packet[4..4 + payload.len()].copy_from_slice(&payload);
        packet
    }

    fn encode_test_pts(pts: u64) -> [u8; 5] {
        [
            0x20 | (((pts >> 30) as u8 & 0x07) << 1) | 1,
            ((pts >> 22) & 0xff) as u8,
            ((((pts >> 15) as u8) & 0x7f) << 1) | 1,
            ((pts >> 7) & 0xff) as u8,
            (((pts as u8) & 0x7f) << 1) | 1,
        ]
    }
}
