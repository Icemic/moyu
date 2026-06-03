<h1 align="center">Moyu - Cross-Platform Visual Novel Engine</h1>
<img alt="Momoyu Logo" src="https://repository-images.githubusercontent.com/456076228/ecb67727-0821-47c9-aef2-9634f58f3e92" width="50%" style="display: block; margin: 0 auto;">
<br>
<p align="center"><a href="README.md">简体中文</a> | <strong>English</strong> | <a href="README_JP.md">日本語</a></p>
<p align="center">A progressive cross-platform visual novel engine with a Rust core, using browser-independent React to build interfaces and presentation.</p>
<p align="center">
  <a href="https://opensource.org/licenses/MPL-2.0"><img alt="MPL-2.0 License" src="https://img.shields.io/badge/license-MPL%202.0-blue.svg"></a>
  <a href="https://github.com/Icemic/moyu/actions"><img alt="Rust CI" src="https://github.com/Icemic/moyu/actions/workflows/build.yml/badge.svg"></a>
  <a href="https://github.com/Icemic/moyu/pulls"><img alt="PRs Welcome" src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat"></a>
  <a href="https://discord.gg/wmTekCNarG"><img alt="Discord" src="https://img.shields.io/discord/1260706796646170765?label=Discord&logo=discord&logoColor=white"></a>
  <a href="http://qm.qq.com/cgi-bin/qm/qr?_wv=1027&k=dcB58s03NbyIENYYtp0IHa8aTcUzlBF4&authKey=cgKWlgzqOhczlLbJbGo%2F1wLiUzH%2FMXNSTxz%2BNhDjMufuw0egSin7eqZKoRD7vF4l&noverify=0&group_code=293602841"><img alt="QQ" src="https://img.shields.io/badge/QQ-293602841-blue?logo=qq&logoColor=white"></a>
</p>

<hr>

Built for modern visual novel development, Moyu combines a Rust core with a JS/React development workflow, offering creators a progressive experience from rapid prototyping to deep customization.

For a complete introduction, tutorials, and documentation, visit the official site: <https://momoyu.ink>.

## Features

- **Consistent cross-platform support**: Runs on Windows, macOS, Linux, Android, iOS, and Web — write once, run everywhere.
- **Multiple graphics backends**: Switch between Vulkan / Metal / DX12 / OpenGL.
- **Highly customizable interfaces**: Use React to build any UI and system, reusing the mature ecosystem's resources and tooling.
- **Progressive and flexible**: From the standard framework to deep extensions in the Rust layer, complexity is unlocked step by step.
- **Open source and business-friendly**: Licensed under MPL-2.0 — free to use, including for commercial projects.

### Layered Architecture

- **Rust layer**: Resource management / graphics rendering / audio system / native plugins.
- **JavaScript layer**: React components / story logic / animation orchestration.

## Repository Structure

- `crates/` — The Rust engine core, runtime, nodes, platform abstraction, and more.
- `packages/` — Upper-level JavaScript / TypeScript: `@momoyu-ink/kit` (React SDK), `@momoyu-ink/cli` (CLI), and more.

## Quick Start

The engine is used through its companion standard framework. Head to the official framework repository and follow its instructions to clone, install, and run:

<https://github.com/DeepSpaceMill/framework>

For complete guidance on installation, asset placement, and script writing, see the official site: <https://momoyu.ink>.

## Contributing

Contributions of all kinds are welcome, including documentation improvements and internationalization, template development, engine feature extensions, new platform support, and performance optimization. Please read the [Contributing Guide](CONTRIBUTING.md) before getting started.

## Community

If you have questions or ideas, feel free to join the conversation on Discord or in the QQ group.

## License

Unless otherwise specified, this project is licensed under the Mozilla Public License v2.0 (MPL-2.0), as described in each `Cargo.toml` or `package.json` file and the [LICENSE](LICENSE.txt) file.

Some content is licensed under the MIT License; see the `package.json` and LICENSE files in the corresponding directories for details.
