<h1 align="center">末语 - 跨平台视觉小说引擎</h1>
<img alt="Momoyu Logo" src="https://repository-images.githubusercontent.com/456076228/ecb67727-0821-47c9-aef2-9634f58f3e92" width="50%" style="display: block; margin: 0 auto;">
<br>
<p align="center"><strong>简体中文</strong> | <a href="README_EN.md">English</a> | <a href="README_JP.md">日本語</a></p>
<p align="center">渐进式跨平台视觉小说引擎，以 Rust 为核心，使用脱离浏览器的 React 构建界面与演出。</p>
<p align="center">
  <a href="https://opensource.org/licenses/MPL-2.0"><img alt="MPL-2.0 License" src="https://img.shields.io/badge/license-MPL%202.0-blue.svg"></a>
  <a href="https://github.com/Icemic/moyu/actions"><img alt="Rust CI" src="https://github.com/Icemic/moyu/actions/workflows/build.yml/badge.svg"></a>
  <a href="https://github.com/Icemic/moyu/pulls"><img alt="PRs Welcome" src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat"></a>
  <a href="https://discord.gg/wmTekCNarG"><img alt="Discord" src="https://img.shields.io/discord/1260706796646170765?label=Discord&logo=discord&logoColor=white"></a>
  <a href="http://qm.qq.com/cgi-bin/qm/qr?_wv=1027&k=dcB58s03NbyIENYYtp0IHa8aTcUzlBF4&authKey=cgKWlgzqOhczlLbJbGo%2F1wLiUzH%2FMXNSTxz%2BNhDjMufuw0egSin7eqZKoRD7vF4l&noverify=0&group_code=293602841"><img alt="QQ" src="https://img.shields.io/badge/QQ-293602841-blue?logo=qq&logoColor=white"></a>
</p>

<hr>

面向现代视觉小说开发，基于 Rust 核心与 JS/React 开发范式，为创作者提供从快速原型到深度定制的渐进式体验。

更完整的介绍、教程与文档请访问官网：<https://momoyu.ink>。

## 特性

- **一致的跨平台能力**：支持 Windows、macOS、Linux、Android、iOS 与 Web，一次编写，各处运行。
- **多种图形后端**：支持切换 Vulkan/Metal/DX12/OpenGL
- **高度自定义的界面**：使用 React 定制任意界面与系统，复用成熟社区的资源与工具链。
- **渐进式与灵活性**：从标准框架到深入 Rust 层的底层扩展，逐级开放复杂度。
- **开源且商业友好**：基于 MPL-2.0 协议，可免费使用，亦可用于商业项目。

### 分层架构

- **Rust 底层**：资源管理/图形渲染/音频系统/原生插件
- **JavaScript 上层**：React 组件化/剧情逻辑/动画编排

## 仓库结构

- `crates/` — Rust 实现的引擎核心、运行时、节点、平台抽象等。
- `packages/` — 上层 JavaScript / TypeScript：`@momoyu-ink/kit`（React SDK）、`@momoyu-ink/cli`（CLI）等。

## 快速开始

引擎本体通过配套的标准框架使用。前往官方框架仓库，按其说明克隆、安装并运行：

<https://github.com/DeepSpaceMill/framework>

完整的安装、资源放置与剧本编写指引见官网：<https://momoyu.ink>。

## 参与贡献

欢迎文档改进与国际化、模板开发、引擎功能扩展、新平台适配、性能优化等各类贡献。开始前请阅读[贡献指南](CONTRIBUTING.md)。

## 社区

如有问题或想法，欢迎在 Discord 或 QQ 群参与交流。

## 开源许可

除非另有说明，本项目遵循 Mozilla Public License v2.0（MPL-2.0），具体见各 `Cargo.toml` 或 `package.json` 文件及 [LICENSE](LICENSE.txt)。

部分内容遵循 MIT 协议，详见对应目录下的 `package.json` 与 LICENSE 文件。
