# 播放格式支持

## 策略

项目使用多个播放引擎，而不是假设一个浏览器播放器能播放所有本地文件。

```text
内置播放器：启动快、界面融合好
mpegts.js：优先支持本地 .ts 文件
hls.js：支持 HLS 播放列表
EasyPlayer.js：重点评估的增强播放器，覆盖 HLS、HTTP-FLV、fMP4、WebRTC、H.265 等场景
mpv：主流本地文件格式兼容兜底
```

## 格式矩阵

| 格式 | 优先级 | 优先引擎 | 兜底 | 说明 |
| --- | --- | --- | --- | --- |
| `.ts` | P0 | mpegts.js | mpv | 第一优先级格式 |
| `.m2ts`, `.mts` | P0 | mpegts.js | mpv | 取决于容器内编码 |
| `.m3u8` | P1 | EasyPlayer.js 或 hls.js | mpv | HLS 播放列表，常引用 `.ts` 分片 |
| HTTP-FLV, fMP4, WebRTC, H.265 流 | P1 | EasyPlayer.js | mpv 或外部方案 | EasyPlayer.js 的强项，许可证确认后接入 |
| `.mp4`, `.m4v` | P1 | HTML5 视频 | mpv | H.264/AAC 兼容性最好 |
| `.sz` | P1 | FFmpeg 转码缓存 + HTML5 视频 | 明确诊断 | 先按 MP4/TS 等真实容器尝试转码；如果音视频帧不是标准 H.264/AAC，则给出具体诊断 |
| `.webm`, `.ogv` | P2 | HTML5 视频 | mpv | 取决于桌面网页视图的编码支持 |
| `.mp3`, `.wav`, `.ogg`, `.flac`, `.m4a`, `.aac`, `.opus`, `.wma` | P2 | HTML5 音频 | mpv | 已接入音频播放面板，具体编码取决于 WebView |
| `.mkv` | P1 | mpv | 外部打开 | 内置播放器不可靠 |
| `.avi`, `.flv`, `.mov`, `.wmv`, `.3gp`, `.mpeg`, `.mpg` | P2 | mpv | 外部打开 | 兼容路径 |
| `.rmvb`, `.vob` | P3 | mpv | 外部打开 | 尽力支持 |
| `.srt`, `.ass`, `.vtt` | P1 | mpv | 字幕能力范围内支持 | 按同名文件自动匹配 |

## 引擎选择

播放调度层按下面顺序选择：

```text
.ts/.m2ts/.mts
  -> 本地媒体地址 + mpegts.js
  -> 内置播放器适配器失败后切 mpv

.m3u8
  -> EasyPlayer.js 或 hls.js
  -> 内置播放器适配器失败后切 mpv

HTTP-FLV/fMP4/WebRTC/H.265 流
  -> EasyPlayer.js
  -> 失败后给出清楚错误或切换兜底方案

.mp4/.webm
  -> HTML5 video
  -> 解码或加载失败后切 mpv

.sz
  -> 先按真实文件头识别，有些是伪装扩展名的 MP4
  -> 调用 FFmpeg 转为 H.264/AAC/yuv420p 兼容 MP4 缓存
  -> 命中缓存时复用缓存文件
  -> 如果容器像 MP4 但 H.264/AAC 帧无法解码，提示具体码流诊断
  -> 不做未授权的密钥提取、绕过授权或专有解密

.mp3/.wav/.ogg/.flac
  -> HTML5 audio + 音频面板
  -> 解码或加载失败后切 mpv

其他主流视频
  -> mpv
```

## `.ts` 完成标准

`.ts` 支持必须满足：

- 本地 `.ts` 文件可以开始播放。
- 大文件通过 HTTP 分段请求加载。
- 可以拖动进度。
- 可以暂停和恢复。
- 技术条件允许时展示时长或进度。
- 播放失败时可以切到 mpv。
- 测试过 H.264/AAC 的 `.ts`。
- H.265 或异常音频编码能给出清楚兜底或错误提示。

## 内置播放器要求

- 切换文件时必须销毁上一轮 `mpegts.js` 或 `hls.js` 实例。
- 避免内存泄漏。
- 解码、网络、不支持格式等错误必须反馈给用户。
- 媒体地址不能暴露本地绝对路径。
- 使用后端签发的本地地址，不直接使用原始文件路径。

## mpv 要求

- 只能用后端校验过的本地文件路径启动 mpv。
- mpv 生命周期必须可控：启动、停止、重启、销毁。
- 进程通信命令必须结构化。
- 进度事件要节流。
- 字幕自动加载优先匹配同名字幕。
- 如果随包分发 mpv，必须记录版本和许可证。

## FFmpeg 转码缓存要求

- FFmpeg 只生成兼容播放缓存，不替代原始文件。
- 转码输出写入系统临时缓存目录，不修改用户课程文件。
- `.sz` 选中后直接转为兼容 MP4：优先 1080P H.264/AAC 重编码，失败后尝试 720P 保守重编码和快速重封装。
- 缓存按原文件路径、大小和修改时间生成，命中缓存时不得重复转码。
- FFmpeg 查找顺序：`OPEN_COURSE_PLAYER_FFMPEG` 环境变量、应用目录/项目目录、`src-tauri/binaries`、`@ffmpeg-installer/ffmpeg`、Windows 格式工厂常见安装目录、系统 `PATH`。
- 如果随包分发 FFmpeg，必须记录版本、构建参数、许可证和源码获取方式。
- `.sz` 不是统一开放格式。如果 FFmpeg 报 `Invalid NAL unit size`、`Error splitting the input into NAL units`、AAC 通道配置异常等错误，表示 MP4 容器内的音视频帧不是标准 H.264/AAC 码流；这时需要专用解码器、厂商 SDK、官方导出 MP4，或明确且有授权的格式说明。

## EasyPlayer.js 要求

- EasyPlayer.js 是重点评估播放器，不是弃用方案。
- 优先用于 HLS、HTTP-FLV、fMP4、WebRTC、H.265、WASM/WebCodec 解码等增强场景。
- 公开发布前必须确认许可证、分发方式和是否允许随安装包打包。
- 如果许可证确认可用，应把它作为 `EasyPlayerAdapter` 接入播放调度层。
- 如果许可证或分发规则不清楚，可以先保留本地预研，不进入公开安装包。

## 不支持格式行为

不支持文件只能处于这些状态之一：

- 根据用户过滤设置隐藏。
- 显示但禁用。
- 显示并提供“外部打开”。
- 通过 mpv 尽力播放。

用户点击一个已知媒体扩展名时，应用不能静默失败。
