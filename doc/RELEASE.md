# 发布流程

## 发布目标

初始发布目标：

```text
Windows x64 安装包
```

macOS 和 Linux 是后续目标，必须分别验证播放和打包后才能发布。

## 发布仓库

源码仓库和 Release 发布目标：

```text
https://github.com/11477872997/open-course-player.git
```

Git remote 建议：

```powershell
git remote add origin https://github.com/11477872997/open-course-player.git
```

公开版本统一通过 GitHub Releases 发布。每次正式发布必须有对应 tag、发布说明、安装包和校验和。

## 构建命令

正式命令在项目实现后确定。预期形态：

```powershell
pnpm install
pnpm run typecheck
pnpm run build
pnpm run tauri:build
```

## 安装包内容

安装包只能包含已记录且必要的文件：

- Tauri 应用可执行文件。
- 前端构建产物。
- 运行时配置。
- 可选 mpv 随包程序和必要文件。
- 第三方许可证文件。
- 项目许可证、署名声明和第三方声明。

## 版本规则

使用语义化版本：

```text
主版本.次版本.修订版本
```

- 主版本：不兼容的数据或行为变化。
- 次版本：新增功能或格式支持。
- 修订版本：修复和小兼容性改进。

Git tag 使用 `v` 前缀：

```text
v0.1.0
v0.2.0
v1.0.0
```

## 发布前检查

- 前端类型检查通过。
- 前端构建通过。
- Rust 构建通过。
- `.ts` 播放通过。
- `.mp4` 播放通过。
- 至少一个 mpv 兜底格式播放通过。
- 播放、暂停、拖动、音量、全屏都通过。
- 中文路径、空格路径、长文件名通过。
- 不支持格式提示正常。
- 本地媒体服务确认只绑定 `127.0.0.1`。
- 安装包包含必要许可证和署名声明。
- 生成发布产物校验和。
- 更新 `CHANGELOG.md`。

## 手工播放测试矩阵

每次发布建议测试：

| 场景 | 是否必测 |
| --- | --- |
| `.ts` H.264/AAC | 是 |
| `.ts` 大文件拖动 | 是 |
| `.m3u8` 本地或远程样例 | 声称支持 HLS 前必测 |
| `.mp4` H.264/AAC | 是 |
| `.mkv` 通过 mpv 播放 | 声称支持 mpv 兜底前必测 |
| 中文路径和中文文件名 | 是 |
| 大量文件目录 | 是 |
| 同名 `.srt` 字幕 | 声称支持字幕前必测 |

## 产物命名

推荐格式：

```text
open-course-player_<version>_windows_x64_setup.exe
open-course-player_<version>_windows_x64_setup.exe.sha256
```

## 发布阻塞

出现以下情况不得发布：

- 打包依赖许可证未知。
- 内置 mpv 时缺少 mpv/FFmpeg 声明。
- `.ts` 播放不可用。
- 本地媒体服务可以访问选中根目录之外的文件。
- 安装包含有未记录二进制。
