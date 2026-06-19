# 系统架构

## 总览

`open-course-player` 使用混合播放器架构：

```text
Vue 工作区
  -> 播放调度层
  -> 内置播放器适配器 / mpv 适配器
  -> Tauri 命令调用和事件
  -> Rust 服务
  -> 本地文件系统 / 本地媒体服务 / mpv 进程
```

内置播放器负责启动快、界面一体化的格式；mpv 负责主流本地视频、字幕和复杂编码兜底。

## 技术栈

| 层 | 技术 | 职责 |
| --- | --- | --- |
| 桌面端 | Tauri 2 | 窗口、安装包、命令桥接 |
| 前端 | Vue 3 + TypeScript + Vite | 工作区、目录树、控制条、播放器状态 |
| UI | Element Plus | 树、弹框、表单、菜单 |
| 状态 | Pinia | 目录、播放、设置、进度 |
| 后端 | Rust | 文件访问、媒体服务、mpv 生命周期 |
| 内置播放 | HTML5 视频 + mpegts.js + hls.js | `.ts`、`.m3u8`、`.mp4` 和桌面网页视图兼容格式 |
| 原生播放 | mpv 随包程序 | 主流本地格式兜底 |
| 存储 | SQLite 或本地 JSON | 播放进度、最近目录、设置 |

## 前端主目录

```text
src/views/player/
  PlayerWorkspace.vue
  components/
    FileTree.vue
    PlaybackSurface.vue
    PlayerControls.vue
    PlaylistPanel.vue
    UnsupportedFileState.vue
  composables/
    useFolderTree.ts
    usePlayback.ts
    usePlayerShortcuts.ts

src/player/
  adapters/
    webVideoAdapter.ts
    mpegTsAdapter.ts
    hlsAdapter.ts
    mpvAdapter.ts
  playbackRouter.ts
  mediaTypes.ts

src/api/
  mediaLibrary.ts
  mediaServer.ts
  mpv.ts
  playbackHistory.ts
  playbackProgress.ts

src/store/modules/
  library.ts
  playback.ts
  settings.ts
```

## 后端主模块

```text
src-tauri/src/
  lib.rs
  media_library.rs
  media_server.rs
  media_types.rs
  playback_history.rs
  playback_progress.rs
  mpv.rs
  settings.rs
  db.rs
```

| 模块 | 职责 |
| --- | --- |
| `media_library.rs` | 扫描用户选中的目录，返回安全目录树 |
| `media_server.rs` | 通过 `127.0.0.1` 服务授权文件，并支持分段请求 |
| `media_types.rs` | 扩展名、MIME 和播放类型分类 |
| `playback_history.rs` | 保存资料库路径、当前资料库和最后选择的媒体文件 |
| `playback_progress.rs` | 保存和读取每个文件的播放进度 |
| `mpv.rs` | 启动、停止和控制 mpv 随包程序 |
| `settings.rs` | 应用设置和最近目录 |
| `db.rs` | 如果使用 SQLite，负责连接和迁移 |

## 播放流程

```text
用户点击文件
  -> 根据扩展名和媒体类型分类
  -> 选择优先播放器适配器
  -> 如果选择内置播放，向后端请求本地媒体地址
  -> 启动内置播放器
  -> 如果适配器报告不支持或解码失败，提示或自动切换到 mpv
  -> 周期性保存播放进度
```

## 目录流程

```text
用户选择根目录
  -> Tauri dialog 返回路径
  -> 后端校验路径
  -> 后端扫描允许的文件和文件夹
  -> 前端渲染目录树
  -> 保存为最近目录
```

## 本地媒体服务流程

```text
前端请求媒体地址
  -> 后端把文件标识解析到选中根目录内的文件
  -> 后端返回 http://127.0.0.1:<端口>/media/<访问令牌>
  -> 内置播放器加载媒体地址
  -> 媒体服务校验访问令牌并支持分段请求
```

前端不应该自己用绝对路径拼接媒体服务地址。

## mpv 流程

```text
前端请求通过 mpv 播放
  -> 后端校验文件路径
  -> 后端启动或复用 mpv 进程
  -> 后端连接进程通信通道
  -> 前端通过 Tauri 发送播放/暂停/跳转/音量命令
  -> 后端发出进度和状态事件
```

## 数据存储

早期版本可以先用本地 JSON。等历史、设置和资料库数据变多后，再切 SQLite。

建议持久化内容：

```text
recent_roots
playback_progress
folder_tree_state
user_settings
last_player_engine
```

## 边界规则

- 前端不直接扫描任意本地目录。
- 后端不信任未经授权根目录校验的前端路径。
- 播放适配器不负责目录扫描。
- mpv 进程控制只放在 Rust 后端。
- 新增打包二进制前必须先做开源许可证检查。
