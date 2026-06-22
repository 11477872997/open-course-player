# 第三方声明

这个文件记录计划使用和实际随包分发的第三方组件。

第一次公开二进制发布前，必须把“计划版本”替换成准确版本，并按许可证要求附上许可证文本或链接。

## 计划依赖

| 组件 | 许可证 | 用途 | 是否进入安装包 |
| --- | --- | --- | --- |
| Tauri | MIT 或 Apache-2.0 | 桌面壳和 API 桥接 | 是 |
| Vue | MIT | 前端框架 | 是，编译产物 |
| Element Plus | MIT | UI 组件 | 是，编译产物 |
| mpegts.js | Apache-2.0 | 通过 MSE 播放 MPEG-TS | 是，打包 JS |
| hls.js | Apache-2.0 | HLS 播放 | 是，打包 JS |
| video.js | Apache-2.0 | 可选媒体 UI | 待定 |
| mpv | GPL-2.0-or-later | 原生播放兜底 | 计划 |
| FFmpeg | GPLv3（当前 Windows x64 二进制） | `.sz` 转码为兼容 MP4 | 是，见“当前打包二进制” |
| EasyPlayer.js | 待确认 | 重点评估的增强播放器，流媒体/H.265 | 许可证确认前不进入公开安装包 |

## 打包二进制规则

每个进入安装包的二进制都必须记录：

- 名称。
- 版本。
- 来源地址。
- 许可证。
- 构建或下载方式。
- 是否需要提供源码或源码获取说明。

## 当前打包二进制

| 名称 | 版本 | 来源 | 许可证 | 用途 | 备注 |
| --- | --- | --- | --- | --- | --- |
| FFmpeg Windows x64 binary | `6.1.1 essentials build` | npm 包 `ffmpeg-static@5.3.0`，Windows 构建来自 gyan.dev | GPLv3 | 将 `.sz` 等本地课程视频转为 H.264/AAC/yuv420p 兼容 MP4 缓存 | 通过 `pnpm run ffmpeg:prepare` 复制到 `src-tauri/binaries/ffmpeg.exe`，`@ffmpeg-installer/ffmpeg` 仅作为开发回退 |

## 当前素材

| 素材 | 来源 | 用途 | 授权 |
| --- | --- | --- | --- |
| CD Player.png | 用户提供的本地素材 `C:\Users\11478\Downloads\CD Player.png` | 应用图标、窗口图标、安装包图标 | 由素材提供者确认使用权 |

## 当前开发工具

| 工具 | 来源 | 用途 | 许可证 |
| --- | --- | --- | --- |
| UI/UX Pro Max | `C:\Users\11478\Downloads\ui-ux-pro-max-skill-main\ui-ux-pro-max-skill-main` | 设计系统查询和 UI 规范参考，不进入运行时包 | MIT |

## 许可证检查备注

- 分发 mpv 需要按 GPL 兼容方式发布。
- FFmpeg 许可证取决于构建参数。
- EasyPlayer.js 是重点评估组件，但必须确认许可证后才能随公开安装包发布。

---

# English

This file tracks planned and bundled third-party components.

Before the first public binary release, planned versions must be replaced with exact versions and required license texts or links.

## Planned Dependencies

| Component | License | Use | Bundled in installer |
| --- | --- | --- | --- |
| Tauri | MIT or Apache-2.0 | Desktop shell and API bridge | Yes |
| Vue | MIT | Frontend framework | Yes, compiled assets |
| Element Plus | MIT | UI components | Yes, compiled assets |
| mpegts.js | Apache-2.0 | MPEG-TS playback through MSE | Yes, bundled JS |
| hls.js | Apache-2.0 | HLS playback | Yes, bundled JS |
| video.js | Apache-2.0 | Optional media UI | To be decided |
| mpv | GPL-2.0-or-later | Native playback fallback | Planned |
| FFmpeg | GPLv3 for the current Windows x64 binary | Transcoding `.sz` into compatible MP4 cache | Yes, see "Current Bundled Binaries" |
| EasyPlayer.js | To be confirmed | Key candidate for streaming/H.265 enhancement | No public installer bundling until license is confirmed |

## Current Bundled Binaries

| Name | Version | Source | License | Use | Notes |
| --- | --- | --- | --- | --- | --- |
| FFmpeg Windows x64 binary | `6.1.1 essentials build` | npm package `ffmpeg-static@5.3.0`, Windows build from gyan.dev | GPLv3 | Transcode `.sz` course videos into H.264/AAC/yuv420p compatible MP4 cache | Copied to `src-tauri/binaries/ffmpeg.exe` by `pnpm run ffmpeg:prepare`; `@ffmpeg-installer/ffmpeg` is only a development fallback |

## Current Assets

| Asset | Source | Use | License |
| --- | --- | --- | --- |
| CD Player.png | User-provided local asset `C:\Users\11478\Downloads\CD Player.png` | App icon, window icon, installer icon | Usage rights confirmed by asset provider |

## Current Development Tools

| Tool | Source | Use | License |
| --- | --- | --- | --- |
| UI/UX Pro Max | `C:\Users\11478\Downloads\ui-ux-pro-max-skill-main\ui-ux-pro-max-skill-main` | Design-system query and UI guidance, not bundled at runtime | MIT |
