# 设计系统

本项目使用 `tools/ui-ux-pro-max` 作为设计参考工具，但运行时 UI 不依赖它。

它的作用是帮助生成和校验设计方向；真正落地以本文件和项目代码为准。

## 产品定位

`open-course-player` 是一个本地文件/素材目录播放器，属于桌面生产力工具，不是营销网站。

界面目标：

- 管理多个本地文件文件夹。
- 快速浏览大量视频。
- 优先播放 `.ts`，并为主流视频格式提供兜底。
- 长时间观看时保持稳定、安静、低干扰。

## 设计方向

采用“桌面媒体工作台”风格：

- 左侧：资料库和目录管理。
- 中间：当前目录树或播放列表。
- 右侧：播放区、当前文件信息、播放控制。
- 视觉上接近大厂桌面工具：清晰层级、低饱和色、克制动效、高信息密度。

不采用：

- 营销落地页式 hero。
- 大面积装饰渐变。
- 花哨卡片堆叠。
- 纯黑大块空画布无信息。
- 只支持单个目录的单栏文件树。

## 布局规范

桌面端主布局：

```text
顶栏：产品名、当前资料库、搜索、全局操作
左栏：资料库列表，支持多个文件夹
中栏：当前资料库文件树/播放列表
右侧：播放区 + 当前媒体信息 + 控制条
```

推荐尺寸：

| 区域 | 宽度 |
| --- | --- |
| 资料库栏 | 220px |
| 文件树栏 | 320px - 380px |
| 播放区 | 剩余空间 |
| 顶栏 | 56px |
| 控制条 | 76px |

小屏时：

- 资料库栏和文件树可合并为上方区域。
- 播放区优先占据主要空间。
- 控制条不可被压缩到不可点击。

## 色彩

基础色使用冷静的中性色，强调色使用蓝色。

```css
--ocp-bg: #f5f7fb;
--ocp-surface: #ffffff;
--ocp-surface-2: #f8fafc;
--ocp-border: #d8e0ea;
--ocp-text: #16202b;
--ocp-text-muted: #697684;
--ocp-primary: #2563eb;
--ocp-primary-soft: #eaf1ff;
--ocp-player-bg: #0f141b;
--ocp-player-panel: #171d26;
--ocp-danger: #dc2626;
```

避免整个界面被单一蓝色、紫色、深灰或渐变支配。

## 字体

- 字体族：`Inter`, `Segoe UI`, `Microsoft YaHei`, system-ui。
- 正文：13px - 14px。
- 小标签：12px。
- 面板标题：14px - 16px，字重 600。
- 不使用随视口缩放的字体。

## 组件规范

### 图标

- 使用 Element Plus Icons 或统一 SVG 图标。
- 图标按钮必须有 tooltip。
- 同一层级不混用多套图标风格。
- 不使用 emoji 作为结构性图标。

### 资料库列表

- 支持多个文件夹。
- 每个资料库显示名称、路径尾部、可播放文件数量。
- 当前选中项必须有清晰背景和左侧强调线。
- 支持添加、刷新、后续支持移除。

### 文件树

- 大量文件时信息密度要高。
- 文件名允许省略，但 tooltip 显示全名。
- 播放器类型用低对比标签显示。
- 不支持文件不应占据主要视觉权重。

### 播放区

- 空状态不能只是黑屏。
- 未选择文件时显示清晰提示和主操作。
- 已选择但未播放时显示文件名、路径、引擎、状态。
- 播放区域背景使用深色，但周边信息层次要清楚。

### 控制条

- 控制条按钮点击区域不少于 36px，优先 40px。
- 主要播放按钮突出，其余按钮克制。
- 进度条和音量条不能挤压到不可操作。

## 交互规范

- 添加资料库后自动切换到该资料库。
- 刷新只刷新当前资料库。
- 点击资料库切换当前文件树。
- 点击可播放文件更新右侧当前媒体。
- 点击不支持文件给出明确状态。
- 长任务需要 loading。

## 可访问性

- 正文对比度至少达到 4.5:1。
- 图标按钮必须有 tooltip 或可访问名称。
- 交互控件必须有可见 focus 状态。
- 不依赖颜色单独表达状态。

## 使用 UI/UX Pro Max

本项目内置轻量设计查询工具：

```powershell
python tools/ui-ux-pro-max/scripts/search.py "desktop course video player productivity tool dark content dense" --design-system -p "open-course-player" -f markdown
```

查询 UX 规则：

```powershell
python tools/ui-ux-pro-max/scripts/search.py "desktop player sidebar file tree accessibility" --domain ux -n 6
```

查询颜色：

```powershell
python tools/ui-ux-pro-max/scripts/search.py "productivity video player desktop" --domain color -n 6
```

## 当前设计决策

虽然工具推荐了偏“Video-First Hero / Vibrant”的方向，但本项目不是营销页，而是桌面工具。

因此最终采用：

- 产品类型：桌面生产力工具 + 本地媒体播放器。
- 风格：安静、清晰、高信息密度。
- 色彩：浅色工作区 + 深色播放区 + 蓝色主强调。
- 布局：多资料库三栏工作台。
