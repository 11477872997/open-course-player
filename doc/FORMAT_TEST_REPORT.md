# 格式测试报告

## 重要修正

测试目录不能混入不可播放的假媒体文件。默认测试数据只生成真实可播放或可加载的文件。

如果机器没有安装 `ffmpeg`，脚本只会生成 `playable-wav-3s.wav` 和说明文件。安装 `ffmpeg` 并加入 `PATH` 后，脚本会生成 MP3、MP4、TS、HLS 和字幕样例。

```powershell
powershell -ExecutionPolicy Bypass -File tools/media-fixtures/generate-fixtures.ps1 -Output test-media
```

如果需要测试 mpv 兜底格式的“扫描分类”，再额外加参数：

```powershell
powershell -ExecutionPolicy Bypass -File tools/media-fixtures/generate-fixtures.ps1 -Output test-media -IncludeMpvPlaceholders
```

注意：`-IncludeMpvPlaceholders` 生成的 `sample.mkv` 等文件不是可播放媒体，只用于验证扩展名分类。

## 默认可播放样例

| 文件 | 条件 | 预期 |
| --- | --- | --- |
| `playable-wav-3s.wav` | 无需 ffmpeg | 内置音频可播放，时长约 3 秒 |
| `playable-mp3-3s.mp3` | 需要 ffmpeg | 内置音频可播放，时长约 3 秒 |
| `playable-mp4-5s.mp4` | 需要 ffmpeg | 内置视频可播放，时长约 5 秒 |
| `playable-ts-5s.ts` | 需要 ffmpeg | MPEG-TS 可播放，时长约 5 秒 |
| `playable-hls.m3u8` | 需要 ffmpeg | HLS 路径可加载 |
| `playable-mp4-5s.srt` | 需要 ffmpeg | 与 MP4 同名，自动转 WebVTT 后加载 |
| `playable-ts-5s.srt` | 需要 ffmpeg | 与 TS 同名，自动转 WebVTT 后加载 |

## 当前格式状态

| 格式 | 当前状态 |
| --- | --- |
| `.ts`, `.m2ts`, `.mts` | 内置 mpegts.js + 本地 HTTP Range 服务；后端优先读取真实时长 |
| `.mp3`, `.wav` | 内置音频面板 + 本地 HTTP Range 服务 |
| `.mp4`, `.m4v`, `.webm`, `.ogv` | 内置 video；真实编码取决于系统 WebView |
| `.m3u8` | HLS 路径保留；本地分片需要真实样例验证 |
| `.flac`, `.m4a`, `.aac`, `.opus`, `.ogg`, `.wma` | 已纳入扫描和内置音频尝试；实际解码取决于 WebView |
| `.mkv`, `.avi`, `.flv`, `.mov`, `.wmv`, `.rmvb`, `.vob`, `.3gp`, `.mpeg`, `.mpg` | 走 mpv 兜底；没有 mpv 时会提示安装 mpv |
| `.srt`, `.vtt` | 同名字幕可自动加载到内置 video |
| `.ass` | 已识别；内置浏览器播放器不能直接渲染，建议走 mpv 或后续接字幕渲染库 |
