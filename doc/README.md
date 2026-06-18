# 文档中心

这个目录存放 `open-course-player` 的长期维护文档。

## 执行任务第一原则

不管新增什么功能，都必须先看 [CONSTRAINTS.md](./CONSTRAINTS.md)。

播放器项目会同时碰到本地文件、媒体格式、解码器、子进程、开源许可证和用户隐私。一次变更只有在用户行为、格式支持、安全边界和开源合规都检查过之后，才算真正完成。

## 文档索引

| 文档 | 说明 | 什么时候更新 |
| --- | --- | --- |
| [PROJECT_PLAN.md](./PROJECT_PLAN.md) | 版本阶段、范围和开发顺序 | 路线图或阶段范围变化 |
| [CONSTRAINTS.md](./CONSTRAINTS.md) | 产品、安全、播放、打包约束 | 新能力改变项目边界 |
| [ARCHITECTURE.md](./ARCHITECTURE.md) | 系统分层、模块和数据流 | 模块边界、后端命令、播放流程变化 |
| [PLAYER_SUPPORT.md](./PLAYER_SUPPORT.md) | 格式矩阵和播放器适配策略 | 新增或修改格式支持 |
| [OPEN_SOURCE_COMPLIANCE.md](./OPEN_SOURCE_COMPLIANCE.md) | 许可证策略和第三方义务 | 新增依赖、二进制、解码器或发布资源 |
| [RELEASE.md](./RELEASE.md) | 构建、打包和发布规则 | 构建脚本、安装包内容、发布流程变化 |
| [CHANGE_CHECKLIST.md](./CHANGE_CHECKLIST.md) | 完成变更前的必查清单 | 清单本身不够用时 |

## 必须补文档的情况

- 新增或删除媒体格式支持。
- 新增播放器引擎、解码器、二进制随包程序或原生进程。
- 修改目录扫描或文件访问行为。
- 修改本地 HTTP 媒体服务行为。
- 新增遥测、日志、崩溃上报、自动更新检查或网络请求。
- 修改构建、安装包、自动更新或发布流程。
- 新增许可证未记录的依赖。
- 修复一个以后可能再次踩坑的问题。

## 文档更新顺序

1. 先更新 `doc/` 下对应分类文档。
2. 如果影响项目定位或使用方式，同步更新根目录 `README.md`。
3. 如果影响依赖或安装包内容，同步更新 `THIRD_PARTY_NOTICES.md`。
4. 如果影响用户行为，同步更新 `CHANGELOG.md`。
5. 最后按 [CHANGE_CHECKLIST.md](./CHANGE_CHECKLIST.md) 检查。
