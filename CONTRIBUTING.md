# 贡献指南

感谢你帮助改进 `open-course-player`。

## 开始前

请先阅读：

- [doc/CONSTRAINTS.md](./doc/CONSTRAINTS.md)
- [doc/ARCHITECTURE.md](./doc/ARCHITECTURE.md)
- [doc/PLAYER_SUPPORT.md](./doc/PLAYER_SUPPORT.md)
- [doc/OPEN_SOURCE_COMPLIANCE.md](./doc/OPEN_SOURCE_COMPLIANCE.md)

## 开发原则

- 保持 `.ts` 播放可靠。
- 本地文件访问必须显式、可控、可校验。
- 优先做小而清楚的变更。
- 新增媒体引擎、二进制或依赖时必须补文档。
- 不在没有文档和用户控制的情况下加入遥测或网络行为。

## 合并请求检查

- 变更符合项目约束。
- 相关文档已更新。
- 新依赖已加入 `THIRD_PARTY_NOTICES.md`。
- 播放相关变更附带手工测试说明。
- 应用没有获得超出预期的文件访问能力。

## 代码风格

正式代码风格在应用脚手架创建后确定。

默认方向：

- 前端使用 TypeScript。
- 本地文件、媒体服务、原生进程控制放在 Rust。
- 播放格式判断和播放器调度保持集中。
