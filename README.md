# open-course-player

`open-course-player` 是一个面向本地课程目录和素材目录的开源桌面播放器。

第一目标是 Windows 桌面端优先稳定播放 `.ts` 课程文件；长期目标是提供一个好用的本地课程播放器：左侧目录树，右侧播放区，支持主流视频格式、播放进度、字幕、快捷键和开源合规打包。

## 项目定位

```text
本地目录树 + 课程播放列表 + 桌面媒体播放器
```

本项目不是网盘、爬虫、DRM 破解工具或转码服务。应用只播放用户主动选择的本地目录中的文件。

## 计划技术栈

| 层 | 技术 | 说明 |
| --- | --- | --- |
| 桌面壳 | Tauri 2 | 轻量桌面端打包和 Rust 后端桥接 |
| 前端 | Vue 3 + TypeScript + Vite | 播放器界面、目录树、状态编排 |
| UI | Element Plus | 树、按钮、弹框、设置面板 |
| 后端 | Rust | 目录扫描、本地媒体服务、mpv 进程控制 |
| 内置播放 | HTML5 视频 + mpegts.js + hls.js | `.ts`、`.m3u8`、`.mp4` 等桌面网页视图可播格式 |
| 原生兜底 | mpv 随包程序 | 主流本地视频格式、字幕和复杂编码兜底 |

## 播放策略

| 媒体类型 | 优先方案 | 兜底方案 |
| --- | --- | --- |
| `.ts`, `.m2ts`, `.mts` | 本地 HTTP 媒体服务 + `mpegts.js` | mpv |
| `.m3u8`, HTTP-FLV, fMP4, H.265 流 | EasyPlayer.js / `hls.js` | mpv |
| `.mp4`, `.m4v`, `.webm`, `.mp3`, `.wav` | HTML5 video/audio | mpv |
| `.mkv`, `.avi`, `.flv`, `.mov`, `.wmv`, `.rmvb`, `.vob` | mpv | 外部打开 |
| `.srt`, `.ass`, `.vtt` | mpv 字幕加载 | WebVTT 能力范围内支持 |

## 文档索引

| 文档 | 说明 |
| --- | --- |
| [doc/README.md](./doc/README.md) | 文档中心和更新规则 |
| [doc/PROJECT_PLAN.md](./doc/PROJECT_PLAN.md) | 阶段计划和优先级 |
| [doc/CONSTRAINTS.md](./doc/CONSTRAINTS.md) | 产品、安全、播放、打包约束 |
| [doc/ARCHITECTURE.md](./doc/ARCHITECTURE.md) | 系统架构和模块边界 |
| [doc/PLAYER_SUPPORT.md](./doc/PLAYER_SUPPORT.md) | 格式支持矩阵和播放器调度规则 |
| [doc/OPEN_SOURCE_COMPLIANCE.md](./doc/OPEN_SOURCE_COMPLIANCE.md) | 开源许可证和第三方依赖合规 |
| [doc/RELEASE.md](./doc/RELEASE.md) | 构建、打包、发布流程 |
| [doc/CHANGE_CHECKLIST.md](./doc/CHANGE_CHECKLIST.md) | 每次变更完成前的检查清单 |

## 开源许可证

项目计划采用：

```text
GPL-3.0-or-later
```

这个选择偏保守，原因是项目计划随安装包分发 mpv/FFmpeg 相关播放组件。具体规则见 [doc/OPEN_SOURCE_COMPLIANCE.md](./doc/OPEN_SOURCE_COMPLIANCE.md) 和 [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md)。

EasyPlayer.js 是重点评估的增强播放器，用于 HLS、HTTP-FLV、fMP4、WebRTC、H.265 等场景。它不是被排除，只是公开发布前需要确认许可证和二进制/脚本分发规则。

应用图标使用用户提供的 `CD Player.png`，已生成到 `src-tauri/icons/`。

## 当前状态

当前仓库只包含项目规划、约束、架构和开源合规文档。实现开始前，应先确认这些规则是否符合项目方向。

## 项目仓库

```text
https://github.com/11477872997/open-course-player.git
```

---

# English

`open-course-player` is an open source desktop player for local course folders and media libraries.

The first target is stable Windows playback for `.ts` course files. The long-term goal is a local course player with a folder tree, playback area, mainstream media format support, progress saving, subtitles, shortcuts, and open source compliant packaging.

## Planned Stack

| Layer | Choice | Purpose |
| --- | --- | --- |
| Desktop shell | Tauri 2 | Lightweight desktop packaging and Rust bridge |
| Frontend | Vue 3 + TypeScript + Vite | Player UI, folder tree, state orchestration |
| UI | Element Plus | Tree, buttons, dialogs, settings |
| Backend | Rust | Directory scanning, local media service, mpv process control |
| Built-in playback | HTML5 video + mpegts.js + hls.js | `.ts`, `.m3u8`, `.mp4`, and WebView-compatible formats |
| Native fallback | mpv sidecar | Mainstream local video formats, subtitles, and complex codecs |

## License

The project is planned under `GPL-3.0-or-later` because it may distribute mpv/FFmpeg related playback components.

## Repository

```text
https://github.com/11477872997/open-course-player.git
```
