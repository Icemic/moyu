# 末语 - 跨平台视觉小说引擎

[![MPL-2.0 License](https://img.shields.io/badge/license-MPL%202.0-blue.svg)](https://opensource.org/licenses/MPL-2.0)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat)](https://github.com/Icemic/moyu/pulls)
[![Rust CI](https://github.com/Icemic/moyu/actions/workflows/build.yml/badge.svg)](https://github.com/Icemic/moyu/actions)

[![Discord](https://img.shields.io/discord/1260706796646170765?label=Discord)](https://discord.gg/wmTekCNarG)
[![Telegram](https://img.shields.io/badge/Telegram-Join%20Chat-blue)](https://t.me/moyumoe)
[![QQ群](https://img.shields.io/badge/QQ%E7%BE%A4-293602841-blue)](http://qm.qq.com/cgi-bin/qm/qr?_wv=1027&k=dcB58s03NbyIENYYtp0IHa8aTcUzlBF4&authKey=cgKWlgzqOhczlLbJbGo%2F1wLiUzH%2FMXNSTxz%2BNhDjMufuw0egSin7eqZKoRD7vF4l&noverify=0&group_code=293602841)

**用 React 写视觉小说 | 渐进式跨平台引擎 | MPL-2.0**

面向现代视觉小说开发，基于 Rust 核心与 JS/React 开发范式，为创作者提供从快速原型到深度定制的渐进式体验。

## 核心特性

### 全平台覆盖

- 完整跨平台支持：Windows/macOS/Linux/Android/iOS/Web
- WebGPU 图形后端：支持切换 Vulkan/Metal/DX12/OpenGL
- 主机平台支持（TODO）

### 分层架构，各取所长

- **Rust 底层**：资源管理/图形渲染/音频系统/原生插件
- **JavaScript 上层**：React 组件化/剧情逻辑/动画编排

### 现代化开发

- CLI 工具快速初始化模板
- 热重载与可视化调试（当前仅支持 Web）
- TypeScript 类型支持

## 快速开始

### 安装 CLI

```bash
cargo install moyu-cli
```

### 创建新项目

1. 打开 https://github.com/DeepSpaceMill/template
2. 克隆工程或使用右上角的 "Use this template" 按钮
3. 修改 `package.json` 中的 `name` `description` `author` 字段
4. 运行 `yarn install` 安装依赖

### 运行项目

#### 桌面端

```bash
# 启动本地监听
yarn build -w

# 启动引擎
moyu run
```

#### Web

```bash
# 启动本地监听后访问对应地址
yarn dev
```

## 设计原则

### 渐进式复杂度

- 新手：使用预设模板快速构建剧情
- 进阶：修改 React 组件自定义 UI 和演出
- 专家：直接调用 Rust 层 API，贡献底层功能

### 安全边界

- JS 沙盒限制文件/网络访问
- 异步 I/O 多线程调度
- Rust 内存安全保证

## 加入社区

我们欢迎各类创作者加入社区（链接在开头）：

- 游戏创作者
- Rust 工程师
- Web 工程师
- ...

## 参与贡献

欢迎以下类型贡献：

- 文档改进（特别是国际化支持）
- 测试用例补充
- 新模板开发
- 引擎功能扩展
- 新平台适配
- 性能优化

阅读 [贡献指南](CONTRIBUTING.md) 开始参与。

## 开源许可

MPL-2.0 协议 | 自由使用 商业友好

This project, unless otherwise specified, is subject to the terms of the Mozilla Public License, v. 2.0, as described in each `Cargo.toml` or `package.json` file. For more details, see the [LICENSE](LICENSE.txt) file.

Some of this project is subject to the terms of the MIT License, as described in each of the corresponding `package.json` file. For more details, check the LICENSE file in the corresponding directory.
