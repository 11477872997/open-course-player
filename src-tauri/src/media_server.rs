use serde::Serialize;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    env,
    fs::{self, File},
    hash::{Hash, Hasher},
    io::{BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex, OnceLock},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

static MEDIA_SERVER: OnceLock<Arc<MediaServer>> = OnceLock::new();
static MEDIA_INFO_CACHE: OnceLock<Mutex<HashMap<String, CachedMediaInfo>>> = OnceLock::new();

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
pub struct SzMediaDiagnostic {
    container: String,
    video_codec: Option<String>,
    audio_codec: Option<String>,
    video_sample_count: usize,
    audio_sample_count: usize,
    duration: Option<f64>,
    first_video_sample_size: Option<u32>,
    first_video_sample_head: Option<String>,
    first_audio_sample_size: Option<u32>,
    first_audio_sample_head: Option<String>,
    has_standard_protection_boxes: bool,
    looks_like_standard_h264: bool,
    looks_like_standard_aac: bool,
    mdat_entropy: Option<f64>,
    conclusion: String,
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
    token: String,
    path: PathBuf,
    mime: String,
    size: u64,
    source: MediaSourceKind,
}

#[derive(Clone)]
enum MediaSourceKind {
    File,
    TsVirtual(Vec<TsPacketRun>),
    HlsTs(Vec<TsSegment>),
}

struct MediaServer {
    address: String,
    entries: Mutex<HashMap<String, MediaEntry>>,
}

#[derive(Clone)]
struct CachedMediaInfo {
    modified: Option<SystemTime>,
    size: u64,
    mime: String,
    duration: Option<f64>,
    served_size: u64,
    ts_runs: Option<Vec<TsPacketRun>>,
    ts_segments: Option<Vec<TsSegment>>,
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
    let media_info = cached_media_info(&path, size);
    let duration = media_info.duration;
    let has_hls_segments = media_info
        .ts_segments
        .as_ref()
        .is_some_and(|segments| !segments.is_empty());
    let mime = if has_hls_segments {
        "application/vnd.apple.mpegurl".to_string()
    } else {
        media_info.mime.clone()
    };
    let mut served_size = media_info.served_size;
    let source = if let Some(segments) = media_info.ts_segments {
        MediaSourceKind::HlsTs(segments)
    } else if let Some(runs) = media_info.ts_runs {
        MediaSourceKind::TsVirtual(runs)
    } else {
        MediaSourceKind::File
    };
    let token = media_token(&path, size);
    let mut served_path = path.clone();

    if matches!(&source, MediaSourceKind::File) && mime == "video/mp4" {
        if let Some(faststart_path) = prepare_faststart_mp4(&path)? {
            served_size = faststart_path
                .metadata()
                .map_err(|error| format!("无法读取 MP4 快启动缓存信息：{error}"))?
                .len();
            served_path = faststart_path;
        }
    }

    let server = media_server()?;

    server
        .entries
        .lock()
        .map_err(|_| "媒体服务状态异常".to_string())?
        .insert(
            token.clone(),
            MediaEntry {
                token: token.clone(),
                path: served_path,
                mime: mime.clone(),
                size: served_size,
                source,
            },
        );

    Ok(MediaSourceInfo {
        url: media_url(&server.address, &token, &mime),
        mime,
        size: served_size,
        duration,
    })
}

#[tauri::command]
pub fn transcode_media_to_compatible_mp4(path: String) -> Result<MediaSourceInfo, String> {
    let path = normalize_path(&path);
    let path = PathBuf::from(path)
        .canonicalize()
        .map_err(|error| format!("媒体文件不存在或无法读取：{error}"))?;

    if !path.is_file() {
        return Err("请选择一个媒体文件".to_string());
    }

    let output = compatible_mp4_cache_path(&path)?;
    if !is_valid_cached_file(&output) {
        run_ffmpeg_transcode(&path, &output)?;
    }

    create_media_source(output.to_string_lossy().to_string())
}

#[tauri::command]
pub fn diagnose_sz_media(path: String) -> Result<SzMediaDiagnostic, String> {
    let path = normalize_path(&path);
    let path = PathBuf::from(path)
        .canonicalize()
        .map_err(|error| format!("媒体文件不存在或无法读取：{error}"))?;

    if !path.is_file() {
        return Err("请选择一个媒体文件".to_string());
    }

    diagnose_mp4_like_sz(&path)
}

fn cached_media_info(path: &Path, size: u64) -> CachedMediaInfo {
    let key = display_path(path);
    let modified = path
        .metadata()
        .and_then(|metadata| metadata.modified())
        .ok();
    let cache = MEDIA_INFO_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    if let Ok(cache) = cache.lock() {
        if let Some(info) = cache.get(&key) {
            if info.size == size && info.modified == modified {
                return info.clone();
            }
        }
    }

    let document_mime = document_mime_for_path(path);
    let detected_format = if document_mime.is_some() {
        None
    } else {
        sniff_file_format(path)
    };
    let (ts_runs, ts_segments) = if detected_format == Some(DetectedFormat::MpegTs) {
        if let Ok(data) = std::fs::read(path) {
            let runs = detect_ts_packet_runs(&data);
            let segments = ts_segments_from_runs(&data, &runs);
            (
                (!runs.is_empty()).then_some(runs),
                (!segments.is_empty()).then_some(segments),
            )
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };
    let served_size = ts_runs
        .as_ref()
        .map(|runs| ts_virtual_size(runs))
        .unwrap_or(size);
    let info = CachedMediaInfo {
        modified,
        size,
        mime: document_mime
            .unwrap_or_else(|| mime_for_path(path, detected_format))
            .to_string(),
        duration: ts_segments
            .as_ref()
            .map(|segments| ts_segments_duration(segments))
            .filter(|seconds| *seconds > 0.0)
            .or_else(|| probe_duration_with_runs(path, detected_format, ts_runs.as_deref())),
        served_size,
        ts_runs,
        ts_segments,
    };

    if let Ok(mut cache) = cache.lock() {
        cache.insert(key, info.clone());
    }

    info
}

#[tauri::command]
pub fn find_subtitle_tracks(path: String) -> Result<Vec<SubtitleTrackInfo>, String> {
    let path = PathBuf::from(normalize_path(&path))
        .canonicalize()
        .map_err(|error| format!("媒体文件不存在或无法读取：{error}"))?;
    let parent = path
        .parent()
        .ok_or_else(|| "媒体文件没有所在目录".to_string())?;
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
        "ass" => {
            return Err(
                "ASS 字幕当前不能由内置浏览器播放器直接渲染，可通过 mpv 兜底播放".to_string(),
            )
        }
        _ => return Err("当前字幕格式暂不支持".to_string()),
    };

    let size = source_path
        .metadata()
        .map_err(|error| format!("无法读取字幕文件信息：{error}"))?
        .len();
    let token = media_token(&source_path, size);
    let server = media_server()?;
    server
        .entries
        .lock()
        .map_err(|_| "媒体服务状态异常".to_string())?
        .insert(
            token.clone(),
            MediaEntry {
                token: token.clone(),
                path: source_path,
                mime: "text/vtt; charset=utf-8".to_string(),
                size,
                source: MediaSourceKind::File,
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

    let Some(media_path) = path.strip_prefix("/media/") else {
        write_error_response(&mut stream, 404, "Not Found")?;
        return Ok(());
    };
    let (token, segment_index) = parse_media_request_path(media_path);

    let entry = {
        let entries = server
            .entries
            .lock()
            .map_err(|_| "媒体服务状态异常".to_string())?;
        entries.get(token).cloned()
    };

    let Some(entry) = entry else {
        write_error_response(&mut stream, 404, "Not Found")?;
        return Ok(());
    };

    if let Some(segment_index) = segment_index {
        return serve_hls_segment(
            &mut stream,
            &entry,
            segment_index,
            range_header.as_deref(),
            method == "HEAD",
        );
    }

    serve_media(
        &mut stream,
        &entry,
        range_header.as_deref(),
        method == "HEAD",
    )
}

fn parse_media_request_path(path: &str) -> (&str, Option<usize>) {
    let Some((token, rest)) = path.split_once('/') else {
        return (strip_media_token_suffix(path), None);
    };

    let Some(segment) = rest.strip_prefix("segment/") else {
        return (strip_media_token_suffix(token), None);
    };

    let segment = segment.strip_suffix(".ts").unwrap_or(segment);
    (
        strip_media_token_suffix(token),
        segment.parse::<usize>().ok(),
    )
}

fn strip_media_token_suffix(token: &str) -> &str {
    token
        .split_once('.')
        .map(|(value, _)| value)
        .unwrap_or(token)
}

fn serve_media(
    stream: &mut TcpStream,
    entry: &MediaEntry,
    range_header: Option<&str>,
    head_only: bool,
) -> Result<(), String> {
    if let MediaSourceKind::HlsTs(segments) = &entry.source {
        return serve_hls_playlist(stream, entry, segments, head_only);
    }

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

    if let MediaSourceKind::TsVirtual(runs) = &entry.source {
        return serve_ts_virtual_body(stream, &entry.path, runs, start, content_length);
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

fn serve_hls_playlist(
    stream: &mut TcpStream,
    entry: &MediaEntry,
    segments: &[TsSegment],
    head_only: bool,
) -> Result<(), String> {
    let body = hls_playlist(entry, segments);
    write_memory_response(
        stream,
        "application/vnd.apple.mpegurl; charset=utf-8",
        body.as_bytes(),
        head_only,
    )
}

fn serve_hls_segment(
    stream: &mut TcpStream,
    entry: &MediaEntry,
    index: usize,
    range_header: Option<&str>,
    head_only: bool,
) -> Result<(), String> {
    let MediaSourceKind::HlsTs(segments) = &entry.source else {
        write_error_response(stream, 404, "Not Found")?;
        return Ok(());
    };
    let Some(segment) = segments.get(index) else {
        write_error_response(stream, 404, "Not Found")?;
        return Ok(());
    };

    let (start, end, partial) = parse_range(range_header, segment.size())?;
    let content_length = end.saturating_sub(start).saturating_add(1);
    let status = if partial {
        "HTTP/1.1 206 Partial Content"
    } else {
        "HTTP/1.1 200 OK"
    };
    let headers = format!(
        "{status}\r\n\
         Content-Type: video/mp2t\r\n\
         Content-Length: {content_length}\r\n\
         Accept-Ranges: bytes\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Headers: Range\r\n\
         Access-Control-Expose-Headers: Content-Length, Content-Range, Accept-Ranges\r\n\
         {content_range}\
         Cache-Control: no-cache\r\n\
         Connection: close\r\n\r\n",
        content_range = if partial {
            format!("Content-Range: bytes {start}-{end}/{}\r\n", segment.size())
        } else {
            String::new()
        }
    );
    stream
        .write_all(headers.as_bytes())
        .map_err(|error| format!("failed to write HLS segment headers: {error}"))?;

    if head_only {
        return Ok(());
    }

    let mut file = File::open(&entry.path).map_err(|error| {
        format!(
            "failed to open HLS segment source {}: {error}",
            entry.path.display()
        )
    })?;
    file.seek(SeekFrom::Start(segment.start() as u64 + start))
        .map_err(|error| format!("failed to seek HLS segment source: {error}"))?;

    let mut remaining = content_length;
    let mut buffer = [0u8; 64 * 1024];
    while remaining > 0 {
        let max_read = remaining.min(buffer.len() as u64) as usize;
        let read = file
            .read(&mut buffer[..max_read])
            .map_err(|error| format!("failed to read HLS segment source: {error}"))?;
        if read == 0 {
            break;
        }
        stream
            .write_all(&buffer[..read])
            .map_err(|error| format!("failed to send HLS segment data: {error}"))?;
        remaining = remaining.saturating_sub(read as u64);
    }

    Ok(())
}

fn write_memory_response(
    stream: &mut TcpStream,
    content_type: &str,
    body: &[u8],
    head_only: bool,
) -> Result<(), String> {
    let headers = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Headers: Range\r\n\
         Cache-Control: no-cache\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(headers.as_bytes())
        .map_err(|error| format!("failed to write media response headers: {error}"))?;

    if !head_only {
        stream
            .write_all(body)
            .map_err(|error| format!("failed to write media response body: {error}"))?;
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Mp4Atom {
    start: u64,
    header_size: u64,
    size: u64,
    kind: [u8; 4],
}

#[derive(Clone, Debug)]
struct Mp4TrackSummary {
    codec: String,
    sample_count: usize,
    first_sample_offset: Option<u64>,
    first_sample_size: Option<u32>,
}

fn diagnose_mp4_like_sz(path: &Path) -> Result<SzMediaDiagnostic, String> {
    let mut source =
        File::open(path).map_err(|error| format!("无法打开 .sz 文件用于结构诊断：{error}"))?;
    let file_size = source
        .metadata()
        .map_err(|error| format!("无法读取 .sz 文件信息：{error}"))?
        .len();
    let atoms = read_mp4_top_level_atoms(&mut source, file_size)?;
    let Some(moov) = atoms.iter().copied().find(|atom| &atom.kind == b"moov") else {
        return Ok(SzMediaDiagnostic {
            container: "unknown".to_string(),
            video_codec: None,
            audio_codec: None,
            video_sample_count: 0,
            audio_sample_count: 0,
            duration: None,
            first_video_sample_size: None,
            first_video_sample_head: None,
            first_audio_sample_size: None,
            first_audio_sample_head: None,
            has_standard_protection_boxes: false,
            looks_like_standard_h264: false,
            looks_like_standard_aac: false,
            mdat_entropy: None,
            conclusion: "当前文件不是可识别的 MP4 外壳 .sz".to_string(),
        });
    };
    let Some(mdat) = atoms.iter().copied().find(|atom| &atom.kind == b"mdat") else {
        return Err("当前 .sz 是 MP4 外壳，但没有 mdat 媒体数据".to_string());
    };

    let moov_data = read_file_range(path, moov.start, moov.size)?;
    let duration = mp4_duration_from_moov(&moov_data);
    let tracks = summarize_mp4_tracks(&moov_data);
    let video_track = tracks.iter().find(|track| track.codec == "avc1");
    let audio_track = tracks.iter().find(|track| track.codec == "mp4a");
    let video_head = video_track.and_then(|track| {
        read_sample_head(path, track.first_sample_offset, track.first_sample_size)
    });
    let audio_head = audio_track.and_then(|track| {
        read_sample_head(path, track.first_sample_offset, track.first_sample_size)
    });
    let looks_like_standard_h264 = video_head
        .as_ref()
        .is_some_and(|bytes| looks_like_avcc_h264_sample(bytes));
    let looks_like_standard_aac = audio_head
        .as_ref()
        .is_some_and(|bytes| looks_like_aac_sample(bytes));
    let has_standard_protection_boxes = contains_any_atom_marker(
        &moov_data,
        &[
            b"sinf", b"encv", b"enca", b"pssh", b"tenc", b"senc", b"saio", b"saiz",
        ],
    );
    let entropy_size = (mdat.size.saturating_sub(mdat.header_size)).min(64 * 1024);
    let mdat_entropy = (entropy_size > 0)
        .then(|| read_file_range(path, mdat.start + mdat.header_size, entropy_size).ok())
        .flatten()
        .map(|bytes| byte_entropy(&bytes));
    let conclusion = if video_track.is_some()
        && audio_track.is_some()
        && !looks_like_standard_h264
        && !looks_like_standard_aac
    {
        "MP4 外壳和 avc1/mp4a 轨道表存在，但首段音视频帧不是公开标准 H.264/AAC，常规 FFmpeg 转码无法直接解码。"
            .to_string()
    } else if video_track.is_some() && !looks_like_standard_h264 {
        "MP4 外壳存在，但首段视频帧不是公开标准 H.264 NAL 数据。".to_string()
    } else if audio_track.is_some() && !looks_like_standard_aac {
        "MP4 外壳存在，但首段音频帧不是公开标准 AAC 数据。".to_string()
    } else {
        "结构看起来接近标准 MP4，可继续尝试 FFmpeg 转码。".to_string()
    };

    Ok(SzMediaDiagnostic {
        container: "mp4".to_string(),
        video_codec: video_track.map(|track| track.codec.clone()),
        audio_codec: audio_track.map(|track| track.codec.clone()),
        video_sample_count: video_track.map(|track| track.sample_count).unwrap_or(0),
        audio_sample_count: audio_track.map(|track| track.sample_count).unwrap_or(0),
        duration,
        first_video_sample_size: video_track.and_then(|track| track.first_sample_size),
        first_video_sample_head: video_head.as_ref().map(|bytes| hex_head(bytes)),
        first_audio_sample_size: audio_track.and_then(|track| track.first_sample_size),
        first_audio_sample_head: audio_head.as_ref().map(|bytes| hex_head(bytes)),
        has_standard_protection_boxes,
        looks_like_standard_h264,
        looks_like_standard_aac,
        mdat_entropy,
        conclusion,
    })
}

fn summarize_mp4_tracks(moov_data: &[u8]) -> Vec<Mp4TrackSummary> {
    let mut tracks = Vec::new();
    let moov_body_start = mp4_atom_body_start(moov_data, 0).unwrap_or(8);
    for trak in find_child_atoms(moov_data, moov_body_start, moov_data.len(), *b"trak") {
        let Some(stbl) = find_atom_path(moov_data, trak, &[*b"mdia", *b"minf", *b"stbl"]) else {
            continue;
        };
        let codec = find_child_atoms(
            moov_data,
            atom_body_start(moov_data, stbl),
            stbl.1,
            *b"stsd",
        )
        .first()
        .and_then(|atom| mp4_stsd_codec(moov_data, *atom))
        .unwrap_or_else(|| "unknown".to_string());
        let sample_sizes = find_child_atoms(
            moov_data,
            atom_body_start(moov_data, stbl),
            stbl.1,
            *b"stsz",
        )
        .first()
        .map(|atom| parse_mp4_stsz(moov_data, *atom))
        .unwrap_or_default();
        let chunk_offsets = find_child_atoms(
            moov_data,
            atom_body_start(moov_data, stbl),
            stbl.1,
            *b"stco",
        )
        .first()
        .map(|atom| parse_mp4_stco(moov_data, *atom))
        .unwrap_or_default();
        let stsc = find_child_atoms(
            moov_data,
            atom_body_start(moov_data, stbl),
            stbl.1,
            *b"stsc",
        )
        .first()
        .map(|atom| parse_mp4_stsc(moov_data, *atom))
        .unwrap_or_default();
        let first_sample_offset = first_mp4_sample_offset(&chunk_offsets, &stsc);

        tracks.push(Mp4TrackSummary {
            codec,
            sample_count: sample_sizes.len(),
            first_sample_offset,
            first_sample_size: sample_sizes.first().copied(),
        });
    }

    tracks
}

fn mp4_duration_from_moov(moov_data: &[u8]) -> Option<f64> {
    let body_start = mp4_atom_body_start(moov_data, 0).unwrap_or(8);
    let mvhd = find_child_atoms(moov_data, body_start, moov_data.len(), *b"mvhd")
        .into_iter()
        .next()?;
    let start = atom_body_start(moov_data, mvhd);
    if start >= mvhd.1 || start >= moov_data.len() {
        return None;
    }

    let version = *moov_data.get(start)?;
    let (timescale_offset, duration_offset, duration_size) = if version == 1 {
        (start.checked_add(20)?, start.checked_add(24)?, 8usize)
    } else {
        (start.checked_add(12)?, start.checked_add(16)?, 4usize)
    };

    if timescale_offset.checked_add(4)? > mvhd.1
        || duration_offset.checked_add(duration_size)? > mvhd.1
        || duration_offset.checked_add(duration_size)? > moov_data.len()
    {
        return None;
    }

    let timescale = u32::from_be_bytes(
        moov_data[timescale_offset..timescale_offset + 4]
            .try_into()
            .ok()?,
    );
    if timescale == 0 {
        return None;
    }

    let duration = if duration_size == 8 {
        u64::from_be_bytes(
            moov_data[duration_offset..duration_offset + 8]
                .try_into()
                .ok()?,
        )
    } else {
        u32::from_be_bytes(
            moov_data[duration_offset..duration_offset + 4]
                .try_into()
                .ok()?,
        ) as u64
    };

    let seconds = duration as f64 / timescale as f64;
    seconds.is_finite().then_some(seconds).filter(|value| *value > 0.0)
}

fn find_atom_path(data: &[u8], parent: (usize, usize), path: &[[u8; 4]]) -> Option<(usize, usize)> {
    let mut current = parent;
    for kind in path {
        current = find_child_atoms(data, atom_body_start(data, current), current.1, *kind)
            .into_iter()
            .next()?;
    }
    Some(current)
}

fn find_child_atoms(data: &[u8], start: usize, end: usize, kind: [u8; 4]) -> Vec<(usize, usize)> {
    let mut atoms = Vec::new();
    let mut cursor = start;
    while cursor.saturating_add(8) <= end && cursor.saturating_add(8) <= data.len() {
        let size32 = u32::from_be_bytes(data[cursor..cursor + 4].try_into().unwrap()) as usize;
        let atom_kind = [
            data[cursor + 4],
            data[cursor + 5],
            data[cursor + 6],
            data[cursor + 7],
        ];
        let atom_size = if size32 == 1 {
            if cursor.saturating_add(16) > end || cursor.saturating_add(16) > data.len() {
                break;
            }
            let size64 = u64::from_be_bytes(data[cursor + 8..cursor + 16].try_into().unwrap());
            let Ok(size) = usize::try_from(size64) else {
                break;
            };
            size
        } else if size32 == 0 {
            end.saturating_sub(cursor)
        } else {
            size32
        };

        if atom_size < 8
            || cursor.saturating_add(atom_size) > end
            || cursor.saturating_add(atom_size) > data.len()
        {
            break;
        }

        if atom_kind == kind {
            atoms.push((cursor, cursor + atom_size));
        }
        cursor += atom_size;
    }

    atoms
}

fn atom_body_start(data: &[u8], atom: (usize, usize)) -> usize {
    mp4_atom_body_start(data, atom.0).unwrap_or(atom.0.saturating_add(8))
}

fn mp4_atom_body_start(data: &[u8], start: usize) -> Option<usize> {
    if start.saturating_add(8) > data.len() {
        return None;
    }
    let size32 = u32::from_be_bytes(data[start..start + 4].try_into().ok()?);
    Some(if size32 == 1 { start + 16 } else { start + 8 })
}

fn mp4_stsd_codec(data: &[u8], atom: (usize, usize)) -> Option<String> {
    let body_start = atom_body_start(data, atom);
    let entry_start = body_start.checked_add(8)?;
    if entry_start.checked_add(8)? > atom.1 || entry_start.checked_add(8)? > data.len() {
        return None;
    }
    Some(String::from_utf8_lossy(&data[entry_start + 4..entry_start + 8]).to_string())
}

fn parse_mp4_stsz(data: &[u8], atom: (usize, usize)) -> Vec<u32> {
    let body_start = atom_body_start(data, atom);
    if body_start.saturating_add(12) > atom.1 || body_start.saturating_add(12) > data.len() {
        return Vec::new();
    }
    let sample_size = u32::from_be_bytes(data[body_start + 4..body_start + 8].try_into().unwrap());
    let count =
        u32::from_be_bytes(data[body_start + 8..body_start + 12].try_into().unwrap()) as usize;
    if sample_size > 0 {
        return vec![sample_size; count];
    }
    let table_start = body_start + 12;
    let available = atom
        .1
        .saturating_sub(table_start)
        .min(data.len().saturating_sub(table_start))
        / 4;
    let count = count.min(available);
    (0..count)
        .map(|index| {
            let pos = table_start + index * 4;
            u32::from_be_bytes(data[pos..pos + 4].try_into().unwrap())
        })
        .collect()
}

fn parse_mp4_stco(data: &[u8], atom: (usize, usize)) -> Vec<u64> {
    let body_start = atom_body_start(data, atom);
    if body_start.saturating_add(8) > atom.1 || body_start.saturating_add(8) > data.len() {
        return Vec::new();
    }
    let count =
        u32::from_be_bytes(data[body_start + 4..body_start + 8].try_into().unwrap()) as usize;
    let table_start = body_start + 8;
    let available = atom
        .1
        .saturating_sub(table_start)
        .min(data.len().saturating_sub(table_start))
        / 4;
    let count = count.min(available);
    (0..count)
        .map(|index| {
            let pos = table_start + index * 4;
            u32::from_be_bytes(data[pos..pos + 4].try_into().unwrap()) as u64
        })
        .collect()
}

fn parse_mp4_stsc(data: &[u8], atom: (usize, usize)) -> Vec<(u32, u32, u32)> {
    let body_start = atom_body_start(data, atom);
    if body_start.saturating_add(8) > atom.1 || body_start.saturating_add(8) > data.len() {
        return Vec::new();
    }
    let count =
        u32::from_be_bytes(data[body_start + 4..body_start + 8].try_into().unwrap()) as usize;
    let table_start = body_start + 8;
    let available = atom
        .1
        .saturating_sub(table_start)
        .min(data.len().saturating_sub(table_start))
        / 12;
    let count = count.min(available);
    (0..count)
        .map(|index| {
            let pos = table_start + index * 12;
            (
                u32::from_be_bytes(data[pos..pos + 4].try_into().unwrap()),
                u32::from_be_bytes(data[pos + 4..pos + 8].try_into().unwrap()),
                u32::from_be_bytes(data[pos + 8..pos + 12].try_into().unwrap()),
            )
        })
        .collect()
}

fn first_mp4_sample_offset(chunk_offsets: &[u64], stsc: &[(u32, u32, u32)]) -> Option<u64> {
    if chunk_offsets.is_empty() {
        return None;
    }
    if stsc
        .first()
        .is_some_and(|entry| entry.0 == 1 && entry.1 > 0)
        || stsc.is_empty()
    {
        return chunk_offsets.first().copied();
    }
    chunk_offsets.first().copied()
}

fn read_sample_head(path: &Path, offset: Option<u64>, size: Option<u32>) -> Option<Vec<u8>> {
    let offset = offset?;
    let size = u64::from(size?).min(64);
    read_file_range(path, offset, size).ok()
}

fn looks_like_avcc_h264_sample(bytes: &[u8]) -> bool {
    if bytes.len() < 5 {
        return false;
    }
    let size = u32::from_be_bytes(bytes[0..4].try_into().unwrap()) as usize;
    let nal_type = bytes[4] & 0x1f;
    size > 0 && size <= bytes.len().saturating_sub(4) && matches!(nal_type, 1 | 5 | 6 | 7 | 8 | 9)
}

fn looks_like_aac_sample(bytes: &[u8]) -> bool {
    bytes.len() >= 2 && bytes[0] == 0xff && (bytes[1] & 0xf0) == 0xf0
}

fn contains_any_atom_marker(data: &[u8], markers: &[&[u8; 4]]) -> bool {
    markers
        .iter()
        .any(|marker| data.windows(4).any(|window| window == marker.as_slice()))
}

fn byte_entropy(bytes: &[u8]) -> f64 {
    if bytes.is_empty() {
        return 0.0;
    }
    let mut counts = [0usize; 256];
    for byte in bytes {
        counts[*byte as usize] += 1;
    }
    counts
        .iter()
        .filter(|count| **count > 0)
        .map(|count| {
            let p = *count as f64 / bytes.len() as f64;
            -p * p.log2()
        })
        .sum()
}

fn hex_head(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn prepare_faststart_mp4(path: &Path) -> Result<Option<PathBuf>, String> {
    let mut source =
        File::open(path).map_err(|error| format!("无法打开 MP4 文件用于快启动处理：{error}"))?;
    let file_size = source
        .metadata()
        .map_err(|error| format!("无法读取 MP4 文件信息：{error}"))?
        .len();
    let atoms = read_mp4_top_level_atoms(&mut source, file_size)?;
    let Some(ftyp) = atoms.iter().copied().find(|atom| &atom.kind == b"ftyp") else {
        return Ok(None);
    };
    let Some(moov) = atoms.iter().copied().find(|atom| &atom.kind == b"moov") else {
        return Ok(None);
    };
    let first_mdat_start = atoms
        .iter()
        .filter(|atom| &atom.kind == b"mdat")
        .map(|atom| atom.start)
        .min()
        .unwrap_or(file_size);

    if moov.start < first_mdat_start {
        return Ok(None);
    }

    let cache_path = faststart_cache_path(path, file_size)?;
    if cache_path
        .metadata()
        .map(|metadata| metadata.len() == file_size)
        .unwrap_or(false)
    {
        return Ok(Some(cache_path));
    }

    let mut moov_data = read_file_range(path, moov.start, moov.size)?;
    let offset_delta = i64::try_from(moov.size).map_err(|_| "MP4 moov atom 过大".to_string())?;
    patch_mp4_chunk_offsets(&mut moov_data, offset_delta)?;

    let temp_path = cache_path.with_extension("mp4.tmp");
    let mut writer = BufWriter::new(
        File::create(&temp_path).map_err(|error| format!("无法创建 MP4 快启动缓存：{error}"))?,
    );

    copy_file_range(path, &mut writer, ftyp.start, ftyp.size)?;
    writer
        .write_all(&moov_data)
        .map_err(|error| format!("无法写入 MP4 moov 缓存：{error}"))?;

    for atom in atoms {
        if atom.start == ftyp.start || atom.start == moov.start {
            continue;
        }
        copy_file_range(path, &mut writer, atom.start, atom.size)?;
    }

    writer
        .flush()
        .map_err(|error| format!("无法刷新 MP4 快启动缓存：{error}"))?;
    drop(writer);
    fs::rename(&temp_path, &cache_path).map_err(|error| {
        let _ = fs::remove_file(&temp_path);
        format!("无法保存 MP4 快启动缓存：{error}")
    })?;

    Ok(Some(cache_path))
}

fn read_mp4_top_level_atoms(file: &mut File, file_size: u64) -> Result<Vec<Mp4Atom>, String> {
    let mut atoms = Vec::new();
    let mut cursor = 0u64;
    while cursor.saturating_add(8) <= file_size {
        file.seek(SeekFrom::Start(cursor))
            .map_err(|error| format!("无法定位 MP4 atom：{error}"))?;
        let mut header = [0u8; 16];
        file.read_exact(&mut header[..8])
            .map_err(|error| format!("无法读取 MP4 atom：{error}"))?;
        let size32 = u32::from_be_bytes(header[0..4].try_into().unwrap()) as u64;
        let kind = [header[4], header[5], header[6], header[7]];
        let (size, header_size) = match size32 {
            0 => (file_size.saturating_sub(cursor), 8),
            1 => {
                file.read_exact(&mut header[8..16])
                    .map_err(|error| format!("无法读取 MP4 64 位 atom：{error}"))?;
                (u64::from_be_bytes(header[8..16].try_into().unwrap()), 16)
            }
            value => (value, 8),
        };

        if size < header_size || cursor.saturating_add(size) > file_size {
            break;
        }

        atoms.push(Mp4Atom {
            start: cursor,
            header_size,
            size,
            kind,
        });
        cursor = cursor.saturating_add(size);
    }

    Ok(atoms)
}

fn patch_mp4_chunk_offsets(data: &mut [u8], delta: i64) -> Result<(), String> {
    patch_mp4_container_offsets(data, 0, data.len(), delta)
}

fn patch_mp4_container_offsets(
    data: &mut [u8],
    start: usize,
    end: usize,
    delta: i64,
) -> Result<(), String> {
    let mut cursor = start;
    while cursor.saturating_add(8) <= end {
        let size32 = u32::from_be_bytes(data[cursor..cursor + 4].try_into().unwrap()) as usize;
        let kind = [
            data[cursor + 4],
            data[cursor + 5],
            data[cursor + 6],
            data[cursor + 7],
        ];
        let (atom_size, header_size) = if size32 == 1 {
            if cursor.saturating_add(16) > end {
                break;
            }
            let size64 = u64::from_be_bytes(data[cursor + 8..cursor + 16].try_into().unwrap());
            let atom_size =
                usize::try_from(size64).map_err(|_| "MP4 atom 过大，无法处理快启动".to_string())?;
            (atom_size, 16)
        } else if size32 == 0 {
            (end.saturating_sub(cursor), 8)
        } else {
            (size32, 8)
        };

        if atom_size < header_size || cursor.saturating_add(atom_size) > end {
            break;
        }

        let content_start = cursor + header_size;
        let content_end = cursor + atom_size;
        match &kind {
            b"stco" => patch_stco_offsets(data, content_start, content_end, delta)?,
            b"co64" => patch_co64_offsets(data, content_start, content_end, delta)?,
            _ if is_mp4_container_atom(kind) => {
                patch_mp4_container_offsets(data, content_start, content_end, delta)?;
            }
            _ => {}
        }

        cursor += atom_size;
    }

    Ok(())
}

fn patch_stco_offsets(
    data: &mut [u8],
    content_start: usize,
    content_end: usize,
    delta: i64,
) -> Result<(), String> {
    if content_start.saturating_add(8) > content_end {
        return Ok(());
    }

    let count = u32::from_be_bytes(
        data[content_start + 4..content_start + 8]
            .try_into()
            .unwrap(),
    ) as usize;
    let table_start = content_start + 8;
    for index in 0..count {
        let pos = table_start + index * 4;
        if pos.saturating_add(4) > content_end {
            break;
        }
        let current = u32::from_be_bytes(data[pos..pos + 4].try_into().unwrap()) as i64;
        let updated = current
            .checked_add(delta)
            .ok_or_else(|| "MP4 stco 偏移量溢出".to_string())?;
        let updated =
            u32::try_from(updated).map_err(|_| "MP4 stco 偏移量超出 32 位范围".to_string())?;
        data[pos..pos + 4].copy_from_slice(&updated.to_be_bytes());
    }

    Ok(())
}

fn patch_co64_offsets(
    data: &mut [u8],
    content_start: usize,
    content_end: usize,
    delta: i64,
) -> Result<(), String> {
    if content_start.saturating_add(8) > content_end {
        return Ok(());
    }

    let count = u32::from_be_bytes(
        data[content_start + 4..content_start + 8]
            .try_into()
            .unwrap(),
    ) as usize;
    let table_start = content_start + 8;
    for index in 0..count {
        let pos = table_start + index * 8;
        if pos.saturating_add(8) > content_end {
            break;
        }
        let current = u64::from_be_bytes(data[pos..pos + 8].try_into().unwrap()) as i128;
        let updated = current + delta as i128;
        let updated =
            u64::try_from(updated).map_err(|_| "MP4 co64 偏移量超出 64 位范围".to_string())?;
        data[pos..pos + 8].copy_from_slice(&updated.to_be_bytes());
    }

    Ok(())
}

fn is_mp4_container_atom(kind: [u8; 4]) -> bool {
    matches!(
        &kind,
        b"moov"
            | b"trak"
            | b"mdia"
            | b"minf"
            | b"stbl"
            | b"edts"
            | b"dinf"
            | b"udta"
            | b"meta"
            | b"ilst"
            | b"moof"
            | b"traf"
    )
}

fn faststart_cache_path(path: &Path, size: u64) -> Result<PathBuf, String> {
    let mut hasher = DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    size.hash(&mut hasher);
    path.metadata()
        .and_then(|metadata| metadata.modified())
        .ok()
        .hash(&mut hasher);
    let dir = std::env::temp_dir().join("open-course-player-faststart");
    fs::create_dir_all(&dir).map_err(|error| format!("无法创建 MP4 快启动缓存目录：{error}"))?;
    Ok(dir.join(format!("{:x}.mp4", hasher.finish())))
}

pub(crate) fn prepare_external_playback_path(path: &Path) -> Result<PathBuf, String> {
    let path = path
        .canonicalize()
        .map_err(|error| format!("媒体文件不存在或无法读取：{error}"))?;
    let size = path
        .metadata()
        .map_err(|error| format!("无法读取媒体文件信息：{error}"))?
        .len();
    let detected_format = sniff_file_format(&path);
    if mime_for_path(&path, detected_format) != "video/mp4" {
        return Ok(path);
    }

    if let Some(faststart_path) = prepare_faststart_mp4(&path)? {
        return Ok(faststart_path);
    }

    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("mp4") || ext.eq_ignore_ascii_case("m4v"))
    {
        return Ok(path);
    }

    copy_mp4_external_cache(&path, size)
}

fn copy_mp4_external_cache(path: &Path, size: u64) -> Result<PathBuf, String> {
    let cache_path = faststart_cache_path(path, size)?;
    if cache_path
        .metadata()
        .map(|metadata| metadata.len() == size)
        .unwrap_or(false)
    {
        return Ok(cache_path);
    }

    fs::copy(path, &cache_path).map_err(|error| format!("无法创建外部播放 MP4 缓存：{error}"))?;
    Ok(cache_path)
}

fn compatible_mp4_cache_path(path: &Path) -> Result<PathBuf, String> {
    let size = path
        .metadata()
        .map_err(|error| format!("无法读取媒体文件信息：{error}"))?
        .len();
    let mut hasher = DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    size.hash(&mut hasher);
    path.metadata()
        .and_then(|metadata| metadata.modified())
        .ok()
        .hash(&mut hasher);
    let dir = std::env::temp_dir().join("open-course-player-transcoded");
    fs::create_dir_all(&dir).map_err(|error| format!("无法创建转码缓存目录：{error}"))?;
    Ok(dir.join(format!("{:x}.mp4", hasher.finish())))
}

fn run_ffmpeg_transcode(input: &Path, output: &Path) -> Result<(), String> {
    let strategies = [
        TranscodeStrategy {
            name: "NVIDIA 硬件转码 1080P",
            args: &[
                "-map",
                "0:v:0",
                "-map",
                "0:a?",
                "-map_metadata",
                "0",
                "-vf",
                "scale=w=1920:h=1080:force_original_aspect_ratio=decrease:force_divisible_by=2,setsar=1",
                "-c:v",
                "h264_nvenc",
                "-preset",
                "p4",
                "-cq",
                "23",
                "-profile:v",
                "main",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
                "-b:a",
                "128k",
                "-ac",
                "2",
                "-ar",
                "44100",
                "-af",
                "aresample=async=1:first_pts=0",
                "-avoid_negative_ts",
                "make_zero",
                "-max_muxing_queue_size",
                "2048",
                "-movflags",
                "+faststart",
            ],
        },
        TranscodeStrategy {
            name: "兼容软转码 1080P",
            args: &[
                "-map",
                "0:v:0",
                "-map",
                "0:a?",
                "-map_metadata",
                "0",
                "-vf",
                "scale=w=1920:h=1080:force_original_aspect_ratio=decrease:force_divisible_by=2,setsar=1",
                "-c:v",
                "libx264",
                "-preset",
                "veryfast",
                "-crf",
                "23",
                "-profile:v",
                "main",
                "-level",
                "4.1",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
                "-b:a",
                "128k",
                "-ac",
                "2",
                "-ar",
                "44100",
                "-af",
                "aresample=async=1:first_pts=0",
                "-avoid_negative_ts",
                "make_zero",
                "-max_muxing_queue_size",
                "2048",
                "-movflags",
                "+faststart",
            ],
        },
        TranscodeStrategy {
            name: "保守软转码 720P",
            args: &[
                "-map",
                "0:v:0",
                "-map",
                "0:a?",
                "-map_metadata",
                "0",
                "-vf",
                "scale=w=1280:h=720:force_original_aspect_ratio=decrease:force_divisible_by=2,setsar=1",
                "-c:v",
                "libx264",
                "-preset",
                "faster",
                "-crf",
                "24",
                "-profile:v",
                "main",
                "-level",
                "4.0",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
                "-b:a",
                "128k",
                "-ac",
                "2",
                "-ar",
                "44100",
                "-af",
                "aresample=async=1:first_pts=0",
                "-avoid_negative_ts",
                "make_zero",
                "-max_muxing_queue_size",
                "2048",
                "-movflags",
                "+faststart",
            ],
        },
        TranscodeStrategy {
            name: "快速重封装",
            args: &[
                "-map",
                "0:v:0",
                "-map",
                "0:a?",
                "-map_metadata",
                "0",
                "-c",
                "copy",
                "-bsf:a",
                "aac_adtstoasc",
                "-avoid_negative_ts",
                "make_zero",
                "-movflags",
                "+faststart",
            ],
        },
    ];

    let ffmpeg = ffmpeg_path()?;
    let mut failures = Vec::new();

    for strategy in strategies {
        let temp_output = output.with_extension(format!("{}.mp4.tmp", strategy.name));
        let _ = fs::remove_file(&temp_output);
        let command_output = Command::new(&ffmpeg)
            .args([
                "-hide_banner",
                "-y",
                "-fflags",
                "+genpts+discardcorrupt",
                "-err_detect",
                "ignore_err",
                "-analyzeduration",
                "100M",
                "-probesize",
                "100M",
                "-i",
            ])
            .arg(input)
            .args(strategy.args)
            .arg(&temp_output)
            .output()
            .map_err(|error| {
                format!(
                    "启动 FFmpeg 失败：{error}。请安装 FFmpeg、安装格式工厂，或设置 OPEN_COURSE_PLAYER_FFMPEG 指向 ffmpeg.exe"
                )
            })?;

        if command_output.status.success() && is_valid_cached_file(&temp_output) {
            fs::rename(&temp_output, output)
                .map_err(|error| format!("保存 FFmpeg 转码缓存失败：{error}"))?;
            return Ok(());
        }

        let _ = fs::remove_file(&temp_output);
        let diagnostic = summarize_ffmpeg_failure(&command_output);
        if let Some(diagnostic) = diagnostic.as_ref() {
            failures.push(format!(
                "{}：{}（{}）",
                strategy.name, command_output.status, diagnostic.message
            ));
        } else {
            failures.push(format!("{}：{}", strategy.name, command_output.status));
        }
    }

    if failures
        .iter()
        .any(|item| item.contains("不是标准 H.264 NAL 数据"))
    {
        return Err(format!(
            "FFmpeg 已完整尝试转码和重封装，但当前 .sz 的视频帧不是公开标准 H.264 NAL 数据，音频帧也无法按标准 AAC 解码。已尝试 {}",
            failures.join("；")
        ));
    }

    Err(format!("FFmpeg 转码失败，已尝试 {}", failures.join("；")))
}

struct TranscodeStrategy {
    name: &'static str,
    args: &'static [&'static str],
}

struct FfmpegFailureDiagnostic {
    message: String,
}

fn summarize_ffmpeg_failure(output: &std::process::Output) -> Option<FfmpegFailureDiagnostic> {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stderr}\n{stdout}");
    let lower = combined.to_ascii_lowercase();

    if lower.contains("invalid nal unit size")
        || lower.contains("error splitting the input into nal units")
    {
        return Some(FfmpegFailureDiagnostic {
            message: "文件容器像 MP4，但视频帧不是标准 H.264 NAL 数据".to_string(),
        });
    }

    if lower.contains("invalid data found when processing input")
        || lower.contains("cannot determine format of input stream")
        || lower.contains("inconsistent channel configuration")
    {
        return Some(FfmpegFailureDiagnostic {
            message:
                "FFmpeg 能读到文件头，但音视频码流无法按标准格式解码，可能是专有封装或文件损坏"
                    .to_string(),
        });
    }

    let important_lines = collect_important_ffmpeg_lines(&combined);
    if important_lines.is_empty() {
        return None;
    }

    Some(FfmpegFailureDiagnostic {
        message: important_lines.join(" / "),
    })
}

fn collect_important_ffmpeg_lines(text: &str) -> Vec<String> {
    let patterns = [
        "error",
        "invalid",
        "failed",
        "not found",
        "unsupported",
        "could not",
        "conversion failed",
    ];
    let mut lines = Vec::new();

    for line in text.lines().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let lower = trimmed.to_ascii_lowercase();
        if !patterns.iter().any(|pattern| lower.contains(pattern)) {
            continue;
        }

        let compact = trim_ffmpeg_log_line(trimmed);
        if !lines.iter().any(|line| line == &compact) {
            lines.push(compact);
        }

        if lines.len() >= 3 {
            break;
        }
    }

    lines.reverse();
    lines
}

fn trim_ffmpeg_log_line(line: &str) -> String {
    const MAX_LEN: usize = 160;
    let mut value = line.replace('\t', " ");
    while value.contains("  ") {
        value = value.replace("  ", " ");
    }

    if value.chars().count() <= MAX_LEN {
        return value;
    }

    let mut compact = value.chars().take(MAX_LEN).collect::<String>();
    compact.push_str("...");
    compact
}

fn ffmpeg_path() -> Result<PathBuf, String> {
    if let Some(path) = configured_ffmpeg_path() {
        return Ok(path);
    }

    if let Some(path) = bundled_or_known_ffmpeg_path() {
        return Ok(path);
    }

    if command_exists("ffmpeg") {
        return Ok(PathBuf::from("ffmpeg"));
    }

    Err("未找到 FFmpeg。请安装 FFmpeg、安装格式工厂，或把 ffmpeg.exe 放到项目根目录/bin/src-tauri/binaries；也可以设置 OPEN_COURSE_PLAYER_FFMPEG 指向 ffmpeg.exe".to_string())
}

fn command_exists(name: &str) -> bool {
    Command::new(name)
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn configured_ffmpeg_path() -> Option<PathBuf> {
    env::var_os("OPEN_COURSE_PLAYER_FFMPEG")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
}

fn bundled_or_known_ffmpeg_path() -> Option<PathBuf> {
    ffmpeg_candidates().into_iter().find(|path| path.is_file())
}

fn ffmpeg_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("ffmpeg.exe"));
            candidates.push(dir.join("ffmpeg"));
            candidates.push(dir.join("bin").join("ffmpeg.exe"));
            candidates.push(dir.join("bin").join("ffmpeg"));
        }
    }

    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd.join("ffmpeg.exe"));
        candidates.push(cwd.join("ffmpeg"));
        candidates.push(cwd.join("bin").join("ffmpeg.exe"));
        candidates.push(cwd.join("bin").join("ffmpeg"));
        candidates.push(cwd.join("binaries").join("ffmpeg.exe"));
        candidates.push(cwd.join("binaries").join("ffmpeg"));
        candidates.push(cwd.join("src-tauri").join("binaries").join("ffmpeg.exe"));
        candidates.push(cwd.join("src-tauri").join("binaries").join("ffmpeg"));
        candidates.push(
            cwd.join("node_modules")
                .join("ffmpeg-static")
                .join("ffmpeg.exe"),
        );
        candidates.push(
            cwd.join("node_modules")
                .join("ffmpeg-static")
                .join("ffmpeg"),
        );
        candidates.push(
            cwd.join("node_modules")
                .join("@ffmpeg-installer")
                .join("win32-x64")
                .join("ffmpeg.exe"),
        );
        candidates.push(cwd.join("tools").join("ffmpeg").join("ffmpeg.exe"));
        candidates.push(cwd.join("tools").join("ffmpeg").join("ffmpeg"));
        candidates.extend(find_pnpm_ffmpeg_static_candidates(&cwd));
        candidates.extend(find_pnpm_ffmpeg_installer_candidates(&cwd));

        if let Some(parent) = cwd.parent() {
            candidates.push(parent.join("src-tauri").join("binaries").join("ffmpeg.exe"));
            candidates.push(parent.join("src-tauri").join("binaries").join("ffmpeg"));
            candidates.extend(find_pnpm_ffmpeg_static_candidates(parent));
            candidates.extend(find_pnpm_ffmpeg_installer_candidates(parent));
        }
    }

    #[cfg(target_os = "windows")]
    {
        for key in [
            "ProgramFiles",
            "ProgramFiles(x86)",
            "ProgramW6432",
            "LOCALAPPDATA",
        ] {
            if let Some(root) = env::var_os(key) {
                let root = PathBuf::from(root);
                for base in [
                    root.join("FormatFactory"),
                    root.join("FreeTime").join("FormatFactory"),
                ] {
                    candidates.push(base.join("ffmpeg.exe"));
                    candidates.push(base.join("FFModules").join("ffmpeg.exe"));
                    candidates.push(base.join("FFModules").join("Encoder").join("ffmpeg.exe"));
                }
            }
        }
    }

    candidates
}

fn find_pnpm_ffmpeg_static_candidates(root: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let pnpm_dir = root.join("node_modules").join(".pnpm");
    let Ok(entries) = fs::read_dir(pnpm_dir) else {
        return candidates;
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if !name.starts_with("ffmpeg-static@") {
            continue;
        }

        let base = entry.path().join("node_modules").join("ffmpeg-static");
        candidates.push(base.join("ffmpeg.exe"));
        candidates.push(base.join("ffmpeg"));
    }

    candidates
}

fn find_pnpm_ffmpeg_installer_candidates(root: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let pnpm_dir = root.join("node_modules").join(".pnpm");
    let Ok(entries) = fs::read_dir(pnpm_dir) else {
        return candidates;
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if !name.starts_with("@ffmpeg-installer+") {
            continue;
        }

        let base = entry.path().join("node_modules").join("@ffmpeg-installer");
        candidates.push(base.join("win32-x64").join("ffmpeg.exe"));
        candidates.push(base.join("linux-x64").join("ffmpeg"));
        candidates.push(base.join("darwin-x64").join("ffmpeg"));
        candidates.push(base.join("darwin-arm64").join("ffmpeg"));
    }

    candidates
}

fn is_valid_cached_file(path: &Path) -> bool {
    path.metadata()
        .map(|metadata| metadata.is_file() && metadata.len() > 0)
        .unwrap_or(false)
}

fn read_file_range(path: &Path, start: u64, size: u64) -> Result<Vec<u8>, String> {
    let capacity = usize::try_from(size).map_err(|_| "读取范围过大".to_string())?;
    let mut file = File::open(path).map_err(|error| format!("无法打开文件：{error}"))?;
    file.seek(SeekFrom::Start(start))
        .map_err(|error| format!("无法定位文件：{error}"))?;
    let mut data = vec![0u8; capacity];
    file.read_exact(&mut data)
        .map_err(|error| format!("无法读取文件范围：{error}"))?;
    Ok(data)
}

fn copy_file_range(
    path: &Path,
    writer: &mut BufWriter<File>,
    start: u64,
    size: u64,
) -> Result<(), String> {
    let mut file = File::open(path).map_err(|error| format!("无法打开文件：{error}"))?;
    file.seek(SeekFrom::Start(start))
        .map_err(|error| format!("无法定位文件：{error}"))?;
    let mut remaining = size;
    let mut buffer = [0u8; 256 * 1024];
    while remaining > 0 {
        let max_read = remaining.min(buffer.len() as u64) as usize;
        let read = file
            .read(&mut buffer[..max_read])
            .map_err(|error| format!("无法读取文件：{error}"))?;
        if read == 0 {
            break;
        }
        writer
            .write_all(&buffer[..read])
            .map_err(|error| format!("无法写入缓存文件：{error}"))?;
        remaining = remaining.saturating_sub(read as u64);
    }
    Ok(())
}

fn hls_playlist(entry: &MediaEntry, segments: &[TsSegment]) -> String {
    let target_duration = segments
        .iter()
        .map(|segment| segment.duration.ceil() as u64)
        .max()
        .unwrap_or(1)
        .max(1);
    let mut output = format!(
        "#EXTM3U\n\
         #EXT-X-VERSION:3\n\
         #EXT-X-PLAYLIST-TYPE:VOD\n\
         #EXT-X-TARGETDURATION:{target_duration}\n\
         #EXT-X-MEDIA-SEQUENCE:0\n"
    );

    for (index, segment) in segments.iter().enumerate() {
        if index > 0 {
            output.push_str("#EXT-X-DISCONTINUITY\n");
        }
        output.push_str(&format!("#EXTINF:{:.3},\n", segment.duration.max(0.001)));
        output.push_str(&format!("/media/{}/segment/{index}.ts\n", entry.token));
    }
    output.push_str("#EXT-X-ENDLIST\n");
    output
}

fn serve_ts_virtual_body(
    stream: &mut TcpStream,
    path: &Path,
    runs: &[TsPacketRun],
    start: u64,
    content_length: u64,
) -> Result<(), String> {
    let mut file = File::open(path)
        .map_err(|error| format!("failed to open media file {}: {error}", path.display()))?;
    let mut remaining = content_length;
    let mut virtual_offset = start;
    let mut run_base = 0u64;
    let mut buffer = [0u8; 64 * 1024];

    for run in runs {
        let segment_size = run_size(run);
        if virtual_offset >= run_base.saturating_add(segment_size) {
            run_base = run_base.saturating_add(segment_size);
            continue;
        }

        let mut offset_in_run = virtual_offset.saturating_sub(run_base);
        while offset_in_run < segment_size && remaining > 0 {
            let physical_offset = run.start as u64 + offset_in_run;
            let max_read = (segment_size - offset_in_run)
                .min(remaining)
                .min(buffer.len() as u64) as usize;

            file.seek(SeekFrom::Start(physical_offset))
                .map_err(|error| format!("failed to seek media file: {error}"))?;
            let read = file
                .read(&mut buffer[..max_read])
                .map_err(|error| format!("failed to read media file: {error}"))?;
            if read == 0 {
                return Ok(());
            }

            stream
                .write_all(&buffer[..read])
                .map_err(|error| format!("failed to send media data: {error}"))?;

            let read = read as u64;
            remaining = remaining.saturating_sub(read);
            virtual_offset = virtual_offset.saturating_add(read);
            offset_in_run = offset_in_run.saturating_add(read);
        }

        run_base = run_base.saturating_add(segment_size);
        if remaining == 0 {
            break;
        }
    }

    Ok(())
}

fn run_size(run: &TsPacketRun) -> u64 {
    run.end.saturating_sub(run.start) as u64
}

fn ts_virtual_size(runs: &[TsPacketRun]) -> u64 {
    runs.iter().map(run_size).sum()
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

fn media_url(address: &str, token: &str, mime: &str) -> String {
    let extension = match mime {
        "video/mp4" => ".mp4",
        "video/webm" => ".webm",
        "video/ogg" => ".ogv",
        "audio/mpeg" => ".mp3",
        "audio/wav" => ".wav",
        "audio/ogg" => ".ogg",
        "audio/flac" => ".flac",
        "application/pdf" => ".pdf",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => ".docx",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => ".xlsx",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" => ".pptx",
        "application/vnd.apple.mpegurl" => ".m3u8",
        _ => "",
    };

    format!("http://{address}/media/{token}{extension}")
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DetectedFormat {
    MpegTs,
    Mp4,
    Mp3,
    Wav,
    Flac,
    Ogg,
    Webm,
}

fn mime_for_path(path: &Path, detected_format: Option<DetectedFormat>) -> &'static str {
    if let Some(mime) = document_mime_for_path(path) {
        return mime;
    }

    if let Some(format) = detected_format {
        return mime_for_detected_format(format);
    }

    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "ts" | "m2ts" | "mts" => "video/mp2t",
        "m3u8" => "application/vnd.apple.mpegurl",
        "mp4" | "m4v" | "sz" => "video/mp4",
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

fn document_mime_for_path(path: &Path) -> Option<&'static str> {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "pdf" => Some("application/pdf"),
        "docx" => Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
        "doc" => Some("application/msword"),
        "xlsx" => Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
        "xls" => Some("application/vnd.ms-excel"),
        "pptx" => Some("application/vnd.openxmlformats-officedocument.presentationml.presentation"),
        "ppt" => Some("application/vnd.ms-powerpoint"),
        _ => None,
    }
}

fn mime_for_detected_format(format: DetectedFormat) -> &'static str {
    match format {
        DetectedFormat::MpegTs => "video/mp2t",
        DetectedFormat::Mp4 => "video/mp4",
        DetectedFormat::Webm => "video/webm",
        DetectedFormat::Mp3 => "audio/mpeg",
        DetectedFormat::Wav => "audio/wav",
        DetectedFormat::Flac => "audio/flac",
        DetectedFormat::Ogg => "audio/ogg",
    }
}

fn sniff_file_format(path: &Path) -> Option<DetectedFormat> {
    let mut file = File::open(path).ok()?;
    let mut buffer = [0u8; 64 * 1024];
    let read = file.read(&mut buffer).ok()?;
    let data = &buffer[..read];

    if data.get(0..5) == Some(b"%PDF-") {
        return None;
    }

    if looks_like_mpeg_ts(data) {
        return Some(DetectedFormat::MpegTs);
    }

    if data.len() >= 12 && data.get(4..8) == Some(b"ftyp") {
        return Some(DetectedFormat::Mp4);
    }

    if data.get(0..4) == Some(b"\x1a\x45\xdf\xa3") {
        return Some(DetectedFormat::Webm);
    }

    if data.get(0..3) == Some(b"ID3") || looks_like_mp3_frame(data) {
        return Some(DetectedFormat::Mp3);
    }

    if data.get(0..4) == Some(b"RIFF") && data.get(8..12) == Some(b"WAVE") {
        return Some(DetectedFormat::Wav);
    }

    if data.get(0..4) == Some(b"fLaC") {
        return Some(DetectedFormat::Flac);
    }

    if data.get(0..4) == Some(b"OggS") {
        return Some(DetectedFormat::Ogg);
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
    value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
}

fn probe_duration_with_runs(
    path: &Path,
    detected_format: Option<DetectedFormat>,
    ts_runs: Option<&[TsPacketRun]>,
) -> Option<f64> {
    if detected_format == Some(DetectedFormat::MpegTs) {
        return ts_runs
            .and_then(|runs| probe_ts_duration_from_runs(path, runs))
            .or_else(|| probe_ts_duration(path, detected_format))
            .or_else(|| probe_with_ffprobe(path));
    }

    probe_with_ffprobe(path)
        .or_else(|| probe_wav_duration(path, detected_format))
        .or_else(|| probe_mp3_duration(path, detected_format))
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

fn probe_ts_duration(path: &Path, detected_format: Option<DetectedFormat>) -> Option<f64> {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    if detected_format != Some(DetectedFormat::MpegTs)
        && !matches!(ext.as_str(), "ts" | "m2ts" | "mts")
    {
        return None;
    }

    let data = std::fs::read(path).ok()?;
    let seconds = detect_ts_packet_runs(&data)
        .into_iter()
        .filter_map(|run| ts_run_duration(&data, run))
        .filter(|seconds| *seconds > 0.0 && *seconds < 12.0 * 60.0 * 60.0)
        .sum::<f64>();
    (seconds > 0.0).then_some(seconds)
}

fn probe_ts_duration_from_runs(path: &Path, runs: &[TsPacketRun]) -> Option<f64> {
    let data = std::fs::read(path).ok()?;
    let seconds = runs
        .iter()
        .copied()
        .filter_map(|run| ts_run_duration(&data, run))
        .filter(|seconds| *seconds > 0.0 && *seconds < 12.0 * 60.0 * 60.0)
        .sum::<f64>();
    (seconds > 0.0).then_some(seconds)
}

#[derive(Clone, Copy)]
struct TsPacketRun {
    packet_size: usize,
    start: usize,
    end: usize,
}

#[derive(Clone, Copy)]
struct TsSegment {
    run: TsPacketRun,
    duration: f64,
}

impl TsSegment {
    fn start(&self) -> usize {
        self.run.start
    }

    fn end(&self) -> usize {
        self.run.end
    }

    fn size(&self) -> u64 {
        self.end().saturating_sub(self.start()) as u64
    }
}

fn ts_segments_from_runs(data: &[u8], runs: &[TsPacketRun]) -> Vec<TsSegment> {
    runs.iter()
        .copied()
        .filter_map(|run| {
            let duration = ts_run_duration(data, run)
                .filter(|seconds| *seconds > 0.0 && *seconds < 12.0 * 60.0 * 60.0)
                .unwrap_or_else(|| estimate_ts_run_duration(run));
            (duration > 0.0).then_some(TsSegment { run, duration })
        })
        .collect()
}

fn estimate_ts_run_duration(run: TsPacketRun) -> f64 {
    let packets = run.end.saturating_sub(run.start) / run.packet_size.max(1);
    (packets as f64 / 300.0).max(0.1)
}

fn ts_segments_duration(segments: &[TsSegment]) -> f64 {
    segments.iter().map(|segment| segment.duration).sum()
}

fn detect_ts_packet_runs(data: &[u8]) -> Vec<TsPacketRun> {
    let mut runs = Vec::new();

    for packet_size in [188usize, 192, 204] {
        let mut index = 0usize;
        while index + packet_size <= data.len() {
            if data[index] != 0x47 {
                index += 1;
                continue;
            }

            let start = index;
            let mut count = 0usize;
            while index + packet_size <= data.len() && data[index] == 0x47 {
                count += 1;
                index += packet_size;
            }

            push_ts_run(&mut runs, packet_size, start, count);
            index = index.saturating_add(1);
        }
    }

    runs.sort_by_key(|run| (run.start, usize::MAX - run.end.saturating_sub(run.start)));
    let mut selected: Vec<TsPacketRun> = Vec::new();

    for run in runs {
        if let Some(previous) = selected.last_mut() {
            if run.start < previous.end {
                if run.end.saturating_sub(run.start) > previous.end.saturating_sub(previous.start) {
                    *previous = run;
                }
                continue;
            }
        }

        selected.push(run);
    }

    selected
}

fn push_ts_run(runs: &mut Vec<TsPacketRun>, packet_size: usize, start: usize, count: usize) {
    if count >= 32 {
        runs.push(TsPacketRun {
            packet_size,
            start,
            end: start + count * packet_size,
        });
    }
}

fn ts_run_duration(data: &[u8], run: TsPacketRun) -> Option<f64> {
    let mut pcr_values: HashMap<u16, Vec<f64>> = HashMap::new();
    let mut pts_values: HashMap<u16, Vec<u64>> = HashMap::new();
    let mut index = run.start;

    while index + run.packet_size <= run.end && index + run.packet_size <= data.len() {
        let packet = &data[index..index + run.packet_size];
        if packet.first().copied() == Some(0x47) {
            let pid = ts_packet_pid(packet);
            if let Some(pcr) = parse_ts_packet_pcr(packet) {
                pcr_values.entry(pid).or_default().push(pcr);
            }
            if let Some(pts) = parse_ts_packet_pts(packet) {
                pts_values.entry(pid).or_default().push(pts);
            }
        }
        index += run.packet_size;
    }

    best_pcr_duration(&pcr_values).or_else(|| best_pts_duration(&pts_values))
}

fn ts_packet_pid(packet: &[u8]) -> u16 {
    (((packet.get(1).copied().unwrap_or_default() & 0x1f) as u16) << 8)
        | packet.get(2).copied().unwrap_or_default() as u16
}

fn best_pcr_duration(values: &HashMap<u16, Vec<f64>>) -> Option<f64> {
    values
        .values()
        .filter(|items| items.len() >= 2)
        .filter_map(|items| {
            let min = items.iter().copied().fold(f64::INFINITY, f64::min);
            let max = items.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let seconds = max - min;
            (seconds.is_finite() && seconds > 0.0).then_some(seconds)
        })
        .max_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal))
}

fn best_pts_duration(values: &HashMap<u16, Vec<u64>>) -> Option<f64> {
    values
        .values()
        .filter(|items| items.len() >= 2)
        .filter_map(|items| {
            let first = *items.first()?;
            let mut last = *items.last()?;
            if last < first {
                last += 1u64 << 33;
            }
            let seconds = (last - first) as f64 / 90_000.0;
            (seconds > 0.0).then_some(seconds)
        })
        .max_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal))
}

fn parse_ts_packet_pcr(packet: &[u8]) -> Option<f64> {
    if packet.len() < 12 || packet[0] != 0x47 {
        return None;
    }

    let adaptation_control = (packet[3] >> 4) & 0x03;
    if adaptation_control != 2 && adaptation_control != 3 {
        return None;
    }

    let adaptation_length = packet.get(4).copied()? as usize;
    if adaptation_length < 7 || 5 + adaptation_length > packet.len() {
        return None;
    }

    if packet.get(5).copied()? & 0x10 == 0 {
        return None;
    }

    let pcr = packet.get(6..12)?;
    let base = ((pcr[0] as u64) << 25)
        | ((pcr[1] as u64) << 17)
        | ((pcr[2] as u64) << 9)
        | ((pcr[3] as u64) << 1)
        | ((pcr[4] as u64) >> 7);
    let extension = (((pcr[4] as u64) & 0x01) << 8) | pcr[5] as u64;

    Some(base as f64 / 90_000.0 + extension as f64 / 27_000_000.0)
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

fn probe_wav_duration(path: &Path, detected_format: Option<DetectedFormat>) -> Option<f64> {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    if detected_format != Some(DetectedFormat::Wav) && ext != "wav" {
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
        let chunk_end = chunk_start
            .saturating_add(chunk_size as usize)
            .min(data.len());

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

fn probe_mp3_duration(path: &Path, detected_format: Option<DetectedFormat>) -> Option<f64> {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    if detected_format != Some(DetectedFormat::Mp3) && ext != "mp3" {
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

    if version_id == 1
        || layer == 0
        || bitrate_index == 0
        || bitrate_index == 0x0f
        || sample_rate_index == 3
    {
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
    const V1_L1: [u16; 16] = [
        0, 32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448, 0,
    ];
    const V1_L2: [u16; 16] = [
        0, 32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384, 0,
    ];
    const V1_L3: [u16; 16] = [
        0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0,
    ];
    const V2_L1: [u16; 16] = [
        0, 32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256, 0,
    ];
    const V2_L2L3: [u16; 16] = [
        0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0,
    ];

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

        let ts_path = root.join("video-ts-timestamp-27s.ts");
        let ts = probe_ts_duration(&ts_path, sniff_file_format(&ts_path)).unwrap();
        assert!((ts - 27.0).abs() < 0.2, "ts duration was {ts}");

        let wav_path = root.join("audio-wav-3s.wav");
        let wav = probe_wav_duration(&wav_path, sniff_file_format(&wav_path)).unwrap();
        assert!((wav - 3.0).abs() < 0.2, "wav duration was {wav}");

        let mp3_path = root.join("audio-mp3-3s.mp3");
        let mp3 = probe_mp3_duration(&mp3_path, sniff_file_format(&mp3_path)).unwrap();
        assert!((mp3 - 3.0).abs() < 0.2, "mp3 duration was {mp3}");
    }

    #[test]
    fn sniffs_disguised_course_files() {
        let root = create_test_media_dir("sniff");
        let disguised_ts = root.join("course-audio.mp3");
        write_test_ts(&disguised_ts, 27);
        assert_eq!(
            sniff_file_format(&disguised_ts),
            Some(DetectedFormat::MpegTs)
        );
        assert_eq!(
            mime_for_path(&disguised_ts, sniff_file_format(&disguised_ts)),
            "video/mp2t"
        );

        let disguised_mp4 = root.join("course-video.sz");
        std::fs::write(
            &disguised_mp4,
            [
                0x00, 0x00, 0x00, 0x20, b'f', b't', b'y', b'p', b'i', b's', b'o', b'm', 0x00, 0x00,
                0x02, 0x00,
            ],
        )
        .unwrap();
        assert_eq!(sniff_file_format(&disguised_mp4), Some(DetectedFormat::Mp4));
        assert_eq!(
            mime_for_path(&disguised_mp4, sniff_file_format(&disguised_mp4)),
            "video/mp4"
        );
    }

    #[test]
    fn keeps_pdf_as_document_even_if_bytes_look_audio_like() {
        let root = create_test_media_dir("pdf");
        let pdf = root.join("javascript.pdf");
        std::fs::write(
            &pdf,
            b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog >>\nstream\n\xff\xfb\x90\x64\nendstream\n%%EOF",
        )
        .unwrap();

        assert_eq!(sniff_file_format(&pdf), None);
        assert_eq!(
            mime_for_path(&pdf, sniff_file_format(&pdf)),
            "application/pdf"
        );

        let source = create_media_source(pdf.to_string_lossy().to_string()).unwrap();
        assert_eq!(source.mime, "application/pdf");
        assert!(source.duration.is_none());
    }

    #[test]
    fn media_server_serves_byte_ranges() {
        let root = create_test_media_dir("range");
        let source =
            create_media_source(root.join("audio-wav-3s.wav").to_string_lossy().to_string())
                .unwrap();
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

        assert!(
            response.starts_with("HTTP/1.1 206 Partial Content"),
            "{response}"
        );
        assert!(
            response.contains("Content-Range: bytes 0-15/"),
            "{response}"
        );
        assert!(response.contains("Accept-Ranges: bytes"), "{response}");
    }

    #[test]
    fn media_server_wraps_ts_as_hls_playlist() {
        let root = create_test_media_dir("hls");
        let ts_path = root.join("course.ts");
        let mut data = Vec::new();
        data.extend(test_ts_bytes(6, 40));
        data.extend_from_slice(b"course-file-gap");
        data.extend(test_ts_bytes(8, 40));
        std::fs::write(&ts_path, data).unwrap();

        let source = create_media_source(ts_path.to_string_lossy().to_string()).unwrap();
        assert_eq!(source.mime, "application/vnd.apple.mpegurl");

        let url = source.url.strip_prefix("http://").unwrap();
        let (address, path) = url.split_once('/').unwrap();
        let mut stream = TcpStream::connect(address).unwrap();
        let request =
            format!("GET /{path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n");
        stream.write_all(request.as_bytes()).unwrap();

        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        let response = String::from_utf8_lossy(&response);

        assert!(response.contains("#EXTM3U"), "{response}");
        assert!(response.contains("#EXT-X-DISCONTINUITY"), "{response}");
        assert!(response.contains("/segment/0.ts"), "{response}");
        assert!(response.contains("/segment/1.ts"), "{response}");
    }

    #[test]
    fn media_server_rewrites_tail_moov_mp4_for_faststart() {
        let root = create_test_media_dir("faststart");
        let mp4_path = root.join("course.sz");
        write_tail_moov_mp4(&mp4_path);

        let source = create_media_source(mp4_path.to_string_lossy().to_string()).unwrap();
        assert_eq!(source.mime, "video/mp4");
        assert!(source.url.ends_with(".mp4"), "{}", source.url);

        let url = source.url.strip_prefix("http://").unwrap();
        let (address, path) = url.split_once('/').unwrap();
        let mut stream = TcpStream::connect(address).unwrap();
        let request =
            format!("GET /{path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n");
        stream.write_all(request.as_bytes()).unwrap();

        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        let body_start = response
            .windows(4)
            .position(|bytes| bytes == b"\r\n\r\n")
            .map(|index| index + 4)
            .unwrap();
        let body = &response[body_start..];
        let text = String::from_utf8_lossy(body);
        let moov_index = text.find("moov").unwrap();
        let mdat_index = text.find("mdat").unwrap();
        assert!(moov_index < mdat_index, "{text}");
    }

    #[test]
    fn diagnoses_mp4_shell_with_nonstandard_sz_frames() {
        let root = create_test_media_dir("sz-diagnostic");
        let sz_path = root.join("course.sz");
        write_nonstandard_sz_like_mp4(&sz_path);

        let diagnostic = diagnose_mp4_like_sz(&sz_path).unwrap();
        assert_eq!(diagnostic.container, "mp4");
        assert_eq!(diagnostic.video_codec.as_deref(), Some("avc1"));
        assert_eq!(diagnostic.audio_codec.as_deref(), Some("mp4a"));
        assert!(!diagnostic.looks_like_standard_h264);
        assert!(!diagnostic.looks_like_standard_aac);
        assert!(diagnostic.conclusion.contains("不是公开标准 H.264/AAC"));
        assert!(
            diagnostic.duration.is_some_and(|seconds| (seconds - 492.0).abs() < 0.001),
            "duration was {:?}",
            diagnostic.duration
        );
    }

    #[test]
    fn finds_project_ffmpeg_binary_when_available() {
        let path = ffmpeg_path().unwrap();
        let output = Command::new(path).arg("-version").output().unwrap();
        assert!(output.status.success());
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
        std::fs::write(path, test_ts_bytes(seconds, 40)).unwrap();
    }

    fn write_tail_moov_mp4(path: &Path) {
        let mut data = Vec::new();
        data.extend(atom(b"ftyp", b"isom\0\0\x02\0isomiso2avc1mp41"));
        data.extend(atom(b"free", b""));
        data.extend(atom(b"mdat", &[1, 2, 3, 4, 5, 6, 7, 8]));
        data.extend(atom(
            b"moov",
            &atom(
                b"trak",
                &atom(
                    b"mdia",
                    &atom(
                        b"minf",
                        &atom(
                            b"stbl",
                            &atom(b"stco", &[0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 40]),
                        ),
                    ),
                ),
            ),
        ));
        std::fs::write(path, data).unwrap();
    }

    fn write_nonstandard_sz_like_mp4(path: &Path) {
        let video_sample = vec![0x8a, 0x40, 0x22, 0x87, 0x27, 0x48, 0x55, 0xa7];
        let audio_sample = vec![0xdc, 0xaa, 0x1c, 0x1c, 0xf4, 0x04, 0x18, 0xe3];
        let mut mdat_payload = Vec::new();
        mdat_payload.extend(&video_sample);
        mdat_payload.extend(&audio_sample);
        let audio_offset = 48 + video_sample.len() as u32;

        let video_stbl = atom(
            b"stbl",
            &[
                atom(b"stsd", &stsd_entry(b"avc1")).as_slice(),
                atom(b"stsc", &stsc_one_sample()).as_slice(),
                atom(b"stsz", &stsz_one_sample(video_sample.len() as u32)).as_slice(),
                atom(b"stco", &stco_one_offset(48)).as_slice(),
            ]
            .concat(),
        );
        let audio_stbl = atom(
            b"stbl",
            &[
                atom(b"stsd", &stsd_entry(b"mp4a")).as_slice(),
                atom(b"stsc", &stsc_one_sample()).as_slice(),
                atom(b"stsz", &stsz_one_sample(audio_sample.len() as u32)).as_slice(),
                atom(b"stco", &stco_one_offset(audio_offset)).as_slice(),
            ]
            .concat(),
        );
        let moov = atom(
            b"moov",
            &[
                atom(b"mvhd", &mvhd_duration(1_000, 492_000)).as_slice(),
                atom(b"trak", &atom(b"mdia", &atom(b"minf", &video_stbl))).as_slice(),
                atom(b"trak", &atom(b"mdia", &atom(b"minf", &audio_stbl))).as_slice(),
            ]
            .concat(),
        );

        let mut data = Vec::new();
        data.extend(atom(b"ftyp", b"isom\0\0\x02\0isomiso2avc1mp41"));
        data.extend(atom(b"free", b""));
        data.extend(atom(b"mdat", &mdat_payload));
        data.extend(moov);
        std::fs::write(path, data).unwrap();
    }

    fn stsd_entry(codec: &[u8; 4]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&[0, 0, 0, 0]);
        body.extend_from_slice(&1u32.to_be_bytes());
        body.extend(atom(codec, &[0u8; 8]));
        body
    }

    fn stsc_one_sample() -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&[0, 0, 0, 0]);
        body.extend_from_slice(&1u32.to_be_bytes());
        body.extend_from_slice(&1u32.to_be_bytes());
        body.extend_from_slice(&1u32.to_be_bytes());
        body.extend_from_slice(&1u32.to_be_bytes());
        body
    }

    fn stsz_one_sample(size: u32) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&[0, 0, 0, 0]);
        body.extend_from_slice(&0u32.to_be_bytes());
        body.extend_from_slice(&1u32.to_be_bytes());
        body.extend_from_slice(&size.to_be_bytes());
        body
    }

    fn stco_one_offset(offset: u32) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&[0, 0, 0, 0]);
        body.extend_from_slice(&1u32.to_be_bytes());
        body.extend_from_slice(&offset.to_be_bytes());
        body
    }

    fn mvhd_duration(timescale: u32, duration: u32) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&[0, 0, 0, 0]);
        body.extend_from_slice(&0u32.to_be_bytes());
        body.extend_from_slice(&0u32.to_be_bytes());
        body.extend_from_slice(&timescale.to_be_bytes());
        body.extend_from_slice(&duration.to_be_bytes());
        body
    }

    fn atom(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&((body.len() + 8) as u32).to_be_bytes());
        data.extend_from_slice(kind);
        data.extend_from_slice(body);
        data
    }

    fn test_ts_bytes(seconds: u64, packet_count: u64) -> Vec<u8> {
        let mut data = Vec::new();
        for index in 0..packet_count {
            let pts = seconds * 90_000 * index / (packet_count - 1);
            data.extend_from_slice(&test_ts_packet(pts));
        }
        data
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
