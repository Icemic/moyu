# 贡献指南

欢迎参与豆腐引擎的开发！本指南将帮助您了解如何有效贡献代码、报告问题或提出改进建议。如果你对本项目还不熟悉，可以先阅读[项目文档](https://doufu.moe/docs/)。

## 与社区互动

我们鼓励您通过分享您的想法和反馈来与社区互动。我们有多个官方平台可用于社区参与：

- Discord 或 Telegram 群聊：
  - 与其他开发者交流、寻求帮助或分享项目进展。
  - 共同讨论项目的未来发展和功能需求。
  - 相关链接请查看 [Readme](./README.md) 或 [项目主页](https://doufu.moe/)。
- GitHub Issue：
  - 更严肃的讨论和问题解决。
  - 报告问题或提出改进建议。
  - 提出或参与问题讨论，并希望有持续的记录。
  - 了解项目的开发计划和路线图。
- GitHub Pull Request：
  - 提交代码变更或新功能。
  - 与其他贡献者合作解决问题。
  - 通过代码审查和测试验证您的贡献。
- GitHub Discussions：
  - 与 Discord/Telegram 群聊类似，但更适合长期讨论。
  - 与其他社区成员讨论有关项目的任何主题。
  - 提出问题、分享想法或寻求帮助。
  - 与其他社区成员分享您的项目或资源。

## 开始贡献

### 贡献之前

- 阅读项目文档和代码库，了解项目的目标和架构。
- 在 GitHub 上搜索相关议题，确认您的问题或建议是否已经存在。
- 对于新开发者，我们不建议提交大型功能或架构变更，可以先提出 Issue 讨论或从更小的改进开始。
- 在提交你的 Pull Request 之前，请确保代码至少在本地测试通过。

### 贡献流程

1. 在 [Issue 列表](https://github.com/Icemic/doufu/issues) 中查找或创建相关议题
2. Fork 仓库并创建特性分支（分支命名示例：`feat/audio-xxx` 或 `fix/some-problem`）
3. 提交符合规范的代码变更
4. 创建 Pull Request 并关联相关 Issue
5. 通过 CI 测试和代码审查后合并

### 开发环境

- **Rust 工具链**: 最新稳定版
- **Node.js**: 22.x LTS 版本
- **构建依赖**:
  - 平台指定的构建工具链
  - Clang 18+
  - 对于 Linux，需要 `libsound2-dev` 和 `lld`

## 代码规范

### Rust 代码

- 遵循 [Rust API 指南](https://rust-lang.github.io/api-guidelines/)
- 使用 `cargo fmt` 格式化代码
- 必要的注释和文档

### JavaScript 代码

- 尽量使用简洁易懂 TypeScript
- 遵循我们内置的 ESLint 和 Prettier 规则
- 必要的注释和文档

## 提交规范

### 提交信息格式

遵循 [Conventional Commits](https://www.conventionalcommits.org/) 规范。

## 问题报告

有效的 Bug 报告应包含：

1.  受影响版本号
2.  运行环境（OS/硬件/驱动版本）
3.  重现步骤和代码片段
4.  预期与实际行为
5.  相关日志/截图（如崩溃日志）

## 行为准则

本项目跟随 [Rust 社区行为准则](https://www.rust-lang.org/policies/code-of-conduct)，所有贡献者需遵守交流礼仪和技术伦理规范。
