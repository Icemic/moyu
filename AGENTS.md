# 末语（Moyu）视觉小说引擎 — Agent 指南

本文档面向 AI coding agent 和开发者，阅读并遵守后方可对本仓库进行修改。

本仓库是 **末语（Moyu）** 引擎的核心单体仓库，包含：

- `crates/` — Rust 实现的引擎核心、运行时、节点、平台抽象等
- `packages/` — JavaScript/TypeScript 上层：`@momoyu-ink/kit`（React SDK）、`@momoyu-ink/cli`（CLI）、`gallery`（组件展示与手动测试）、`bunnymark`（基准）

> 关于 `packages/kit` 的详细 SDK 约定，见 [packages/kit/AGENTS.md](packages/kit/AGENTS.md)。
> 关于 `packages/gallery` 的展示应用与资产约定，见 [packages/gallery/AGENTS.md](packages/gallery/AGENTS.md)。
> 关于上层视觉小说框架（基于 kit），见 moyu-framework 仓库的 AGENTS.md。

---

## 通用编码规范

适用于本仓库所有代码（Rust 与 TypeScript）：

- **最小化改动**：实现新功能或修 bug 时，只改必要的部分，不要顺手重构无关代码。
- **从根本上解决问题**：不要打补丁，不要叠床架屋，从根本上分析问题并设计解决方案。
- **不确定就提问**：需要较大重构、需求不明确或存在歧义时，停下来用提问工具向用户确认，不要自行猜测和选择方案。
- **性能意识**：改动涉及渲染循环、JS/Rust 边界、资源加载等热路径时，先评估性能影响。
- **拒绝矛盾需求**：需求之间相互冲突时，拒绝修改并给出详细解释，由用户决定取舍。
- **先读再写**：修改前先阅读相关代码，避免产生重复内容或与既有设计冲突。
- **注释一律英文**：代码文件中的注释必须用英文书写。
- **对话用用户偏好语言**：与用户的自然语言交流使用其偏好语言（默认中文）。
- **检查编译错误**：每次实施完成后检查编辑器报错并按需修复。
- **任务分解**：接到多件事情时，先整体 review + 调研，有问题先问，再按顺序实施；实施完再 review 一遍。
- **合理查询**: 优先使用上下文提供 Tools 进行代码搜索。避免通过命令行遍历文件系统，除非你确切知道要找的文件路径。永远不要在过大的范围内搜索，如整个 `node_modules`、`.cargo/registry/src` 等。

## 文档与规范驱动开发（DDD）

- 优先 **文档与规范驱动**：实现前先写文档、规范或设计方案，讨论后沉淀为 RFCs。放置在 `/rfcs/<date>-<name>.md`。
- 后续变更不应当违反已达成的 RFCs，除非 RFC 本身允许变更或有新的 RFC 取代。
- 当前该流程不是默认要求，只需要在用户明确要求时才执行。
- 当前项目仍然处于从旧的开发模式向 DDD 过渡阶段，部分 RFCs 仍待整理。
- 当你发现本次修改适合用 DDD 流程时，请先创建 RFC 并征求用户意见。

### Rust 构建与测试

- 使用 `cargo build` 检查 Rust 代码。
- **不要**用 `cargo test` 作为一般性检查手段——本仓库不提供单元测试。
- **例外**：根 `package.json` 的 `generate:bindings` 脚本会运行 `cargo test export_bindings --workspace`，这是 `ts-rs` 的绑定导出机制（非单元测试）。只有在需要重新生成 `packages/kit/src/bindings/` 时才运行该脚本。

---

## 技术栈

### 核心依赖

| 组件       | 技术                                        | 用途                       |
| ---------- | ------------------------------------------- | -------------------------- |
| 图形渲染   | `wgpu` + `winit`                            | 跨平台 WebGPU 图形后端     |
| JS 运行时  | QuickJS（native） / 浏览器 JavaScript 运行时（web） | native 侧受限脚本执行，web 侧直接运行入口模块 |
| 音频系统   | `kira` + `symphonia` + `opus`               | 音频播放与解码             |
| 视频解码   | `symphonia`（容器）+ `libloading`（native）/ `WebCodecs`（web） | 视频播放（见 `crates/video`）                   |
| 音频输出（视频）   | `cpal`（native，仅 `moyu_video` 内部） | 视频内嵌音轨的独立输出            |
| 数学库     | `glam`                                      | 矩阵与向量运算             |
| 资源管理   | `image`, `zip`                              | 图片加载与压缩包            |
| 文本排版   | `huozi`                                     | 字体与文本排版             |
| 剧情脚本   | `sixu`                                      | Sixu 脚本解析              |
| TS 绑定    | `ts-rs`                                     | 从 Rust 生成 TypeScript 类型 |
| 异步运行时 | `tokio`（native）/ `wasm-bindgen-futures`（web） | 异步任务调度            |

### 平台条件编译

多个 crate 各自在 `build.rs` 中用 `cfg_aliases` 声明同一套别名（见 `crates/core/build.rs` 等），优先使用别名而非原始 `target_os` / `target_arch`：

```rust
cfg_aliases::cfg_aliases! {
    linux:   { target_os = "linux" },
    macos:   { target_os = "macos" },
    android: { target_os = "android" },
    ios:     { target_os = "ios" },
    wasm:    { target_arch = "wasm32" },

    native:  { any(windows, linux, macos, android, ios) },
    desktop: { any(windows, linux, macos) },
    mobile:  { any(android, ios) },
    web:     { any(wasm) },
}
```

注意：`windows` 是 Rust 内置 `cfg`，不在别名里；别名只有上面这 9 个。新增需要别名的 crate 时记得在该 crate 自己的 `build.rs` 中声明。

---

## 工程结构

### Crates 概览

```
crates/
├── moyu/          # 主入口 crate，整合所有模块，提供可执行入口（main/entry）
├── core/          # 核心引擎：渲染循环、事件系统、节点树、插件系统
├── nodes/         # 内置节点类型：Sprite、Text、Animation、Clip、Filter、Backdrop、Video
├── runtime/       # QuickJS VM 封装与管理、模块加载、console、全局注入
├── ops/           # JS ↔ Rust 桥接操作（节点创建、属性更新、命令分发）
├── resource/      # 资源加载与管理（纹理、字体等），AssetId 与 GC
├── audio/         # 音频播放（Kira + Symphonia + Opus）
├── video/         # 视频播放
├── platform/      # 平台抽象层（别名 moyu_pal）：文件系统、时间、日志等
├── scenario/      # 剧情脚本执行器（Sixu 运行时）
├── gamepad/       # 手柄输入
├── macros/        # 过程宏（Node、Plugin derive 等）
└── run_wasm/      # WASM 构建与开发服务器
```

### Packages 概览

```
packages/
├── kit/           # @momoyu-ink/kit — React SDK，见 packages/kit/AGENTS.md
├── cli/           # @momoyu-ink/cli — 项目初始化、引擎下载/切换、调试运行、打包与 schema 生成命令行
├── gallery/       # 基于 kit + rspack 的组件展示与手动测试应用，见 packages/gallery/AGENTS.md
└── bunnymark/     # 基于 kit + rspack 的性能基准 demo（依赖 @momoyu-ink/kit）
```

### `@momoyu-ink/cli`

CLI 主要用于项目脚手架、引擎资产管理和调试 / 打包流程，当前子命令包括：

- `init` — 初始化 Moyu 项目目录与基础配置
- `download` — 下载指定 channel / version / platform 的引擎资产
- `update` — 在当前 channel 内升级到最新引擎版本，并复用当前项目已下载的平台集合
- `switch` — 切换项目当前激活的已下载引擎版本
- `run` — 以 native 或 web 调试方式运行项目
- `pack` — 使用已下载的引擎资产和项目内容进行打包
- `schema` — 从项目的 Zod 命令定义生成 JSON Schema

具体参数、交互式提示和边界行为请直接查看 `packages/cli/src/commands/*.ts`。

---

## 核心 Crate 详解

### `moyu_core` — 核心引擎

```
core/src/
├── lib.rs           # 模块导出与初始化
├── core.rs          # Core 结构体，引擎主状态
├── state.rs         # 全局状态管理
├── surface.rs       # 窗口与渲染表面
├── base/            # 基础类型（Transform、Point、Vertex 等）
├── core/            # 核心逻辑（渲染、事件处理）
├── events/          # 事件类型定义
├── nodes/           # 节点基类（NodeBase）与容器
├── plugins/         # 内置插件
├── traits/          # 核心 trait
└── utils/           # 工具函数
```

**关键 traits：**

- `Node` — 场景节点，所有可渲染对象必须实现
- `Renderer` — 渲染器，定义特定节点类型的渲染方式
- `Plugin` — 插件，扩展引擎功能
- `Command` — 命令，处理 JS 调用的命令
- `Focusable` — 可聚焦节点

### `moyu_nodes` — 内置节点

当前实现（`crates/nodes/src/nodes/`）：`Sprite`（普通 + 九宫格）/ `Text` / `Animation`（帧动画）/ `Clip` / `Filter` / `Backdrop` / `Video`。

注意：`Container` 属于核心节点，定义在 `moyu_core::nodes::container`，不在 `moyu_nodes` 中。

`moyu_nodes` 还额外暴露：

- `events/` — 节点专属事件类型
- `renderer/` — 对应节点的渲染器实现（`SpriteRenderer` / `TextRenderer` / `AnimationRenderer` / `ClipRenderer` / `BackdropRenderer` / `VideoRenderer` / `OffscreenPassRenderer`）

### `moyu_runtime` — JS 运行时

封装 native 平台的 QuickJS VM（仅 `#[cfg(not(wasm))]`）：

- `QuickVM`（`vm.rs`） — VM 实例与生命周期
- `console`（`console.rs`） — 控制台 API
- `module`（`module.rs`） — 模块加载
- `ops/`（`eval.rs` / `http.rs` / `websocket.rs`） — JS 运行时原语：`__moyu_eval` / `__moyu_fetch` / `__moyu_ws_connect` / `__moyu_ws_send` / `__moyu_ws_close`
- `injections/` — 注入到 VM 的 JS 脚本：`location.js` / `stubs.js` / `websocket.js` / `fetch.js` / `dom.js`

注意：节点相关的桥接操作不在 `moyu_runtime`，而是在 `moyu_ops`。

### `moyu_ops` — 节点/命令桥接

通过 `#[moyu_bindgen]`（native）/ `#[wasm_bindgen]`（web）暴露给 JS 的节点与命令操作（见 `crates/ops/src/node.rs`）：

- `create_instance` / `destroy_instance`
- `add_child` / `insert_child` / `insert_child_before` / `remove_child` / `remove_child_at`
- `update_props`
- `execute_node_command` / `execute_plugin_command`（在 `crates/ops/src/lib.rs` 中注册为 `__moyu_pushCommand` / `__moyu_executeNodeCommand` / `__moyu_executePluginCommand`，并拼装成 JS 端的 `moyu` 全局对象）

另外 `spawn.rs` 提供 `spawn_runtime_with_core`：native 分支负责初始化 QuickJS VM 并执行入口脚本；web 分支则把入口模块注入浏览器文档，并在脚本加载完成后触发 `on_load`。

### `moyu_pal`（`moyu_platform`）— 平台抽象层

- `config` — 配置
- `fs` — 文件系统
- `dir` — 目录路径
- `logger` — 日志
- `time` — 时间
- `task` — 异步任务
- `sync` — 同步原语
- `visible_hand` — 全局状态持有器

### `moyu_scenario` — 剧情脚本执行器

基于 `sixu` 运行 `.sixu` 剧本；提供 `scenario` 插件命令（`getVariable` / `setVariable` / `nextLine` / `terminateStory` 等）。

### `moyu_video` — 视频

独立 crate。容器解析用 `symphonia`；native 平台通过 `libloading` 动态加载外部解码库并用 `cpal` 输出音频，web 平台使用 `WebCodecs` / `VideoDecoder`。最终通过 `<video>` 节点（在 `moyu_nodes`）暴露给上层。

---

## 开发规范

### 创建新节点

1. 在 `crates/nodes/src/nodes/` 下创建文件。
2. 用 `#[derive(Node)]` 宏，包含 `#[base] node_base: NodeBase` 字段。
3. 实现 `Node` trait（必需：`create_instance` + `node_type`；按需覆盖 `update_properties` / `renderer_type` / `as_focusable` / `as_command`）。
4. 在 `crates/nodes/src/renderer/` 添加或扩展对应 renderer。
5. 在 `crates/moyu/src/entry.rs` 的 `ApplicationInitEvent::Plugin` 分支中通过 `core.register_node_type::<T>(name)` 与 `graphics.register_renderer(name, ...)` 注册。
6. 如需在 JS 层使用：给 Props 结构体加 `#[derive(TS)]` → 跑 `yarn generate:bindings` → 在 `packages/kit` 中补 JSX 声明（见 kit 的 AGENTS.md）。

```rust
use moyu_macros::Node;
use moyu_core::nodes::NodeBase;
use moyu_core::traits::{Node, NodeBaseTrait};

#[derive(Debug, Default, Node)]
pub struct MyNode {
    pub my_field: String,

    #[base]
    node_base: NodeBase,
}

impl Node for MyNode {
    fn node_type(&self) -> &'static str { "my_node" }

    fn update_properties(&mut self, props: &mut JSValue) {
        // handle property updates
    }
}
```

### 创建新插件

1. 用 `#[derive(Plugin)]` 宏。
2. 实现 `Plugin` trait。
3. 在 `crates/moyu/src/entry.rs` 注册。

```rust
use moyu_macros::Plugin;
use moyu_core::traits::{Plugin, PluginBaseTrait};

#[derive(Plugin)]
pub struct MyPlugin { /* state */ }

impl Plugin for MyPlugin {
    fn plugin_name(&self) -> &'static str { "my_plugin" }
    fn update(&mut self, vsync: bool) { /* per-frame */ }
}
```

### JS 绑定

使用 `#[moyu_bindgen]` 宏将 Rust 函数暴露给 JS：

```rust
#[moyu_bindgen]
fn my_function(arg: String) -> Result<String> { /* ... */ }
```

### TypeScript 绑定生成

Rust 侧用 `ts-rs` 的 `#[derive(TS)]` 标注需要导出的结构体/枚举。生成命令（仓库根目录）：

```bash
yarn generate:bindings
```

该命令会清空 `packages/kit/src/bindings/`、运行 `cargo test export_bindings --workspace`，再用 `eslint --fix` 格式化。**修改了跨 JS/Rust 边界的类型后务必执行**。

### 资源管理

- 资源通过 `AssetId` 标识
- 由 `ResourceManager` 统一管理
- 支持自动垃圾回收（约 10 秒扫描周期）
- 资源路径相对于 `assets_dir()`

### 事件系统

事件类型位于 `core/src/events/`：

- `GameEvent` — 生命周期
- `KeyboardEvent` / `MouseEvent` / `TouchEvent` / `WheelEvent`
- `NodeEvent`
- `FocusEvent` / `FullScreenEvent` / `ResizeEvent` / `BeforeUnloadEvent`
- `GamepadEvent`
- `AnimationFrameCallbackEvent`（`raf.rs`，支撑 JS 层 `requestAnimationFrame`）
- 自定义事件（`CustomEvent`）

### 错误处理

- Rust 内部用 `anyhow::Result` 传播错误。
- 在 JS 边界返回 `Result<Option<RawJSValue>>`。
- 避免 `panic!`，优先返回错误。

### 线程与生命周期

- 节点、插件需 `Send + Sync`。
- 全局状态的生命周期用 `VisibleHand` 管理。
- 注意 Web 与 Native 的显著差异（如 `tokio` vs `wasm-bindgen-futures`）。

---

## 构建与运行

### Rust

```bash
# 开发构建
cargo build

# 发布构建
cargo build --release

# WASM
cargo build --target wasm32-unknown-unknown

# Android
cargo build --target aarch64-linux-android

# 运行示例
cargo run --example audio -p moyu_audio
cargo run --example simple -p moyu_runtime
```

### JavaScript

根目录使用 **yarn 4**（workspaces）：

```bash
yarn                  # 安装
yarn build            # 构建所有 packages
yarn dev              # 并行开发模式
yarn generate:bindings  # 重新生成 TypeScript bindings
```

---

## 注意事项

1. **平台差异**：大量 `#[cfg(...)]` 条件编译，Web 和 Native 差异显著，改动前确认目标平台范围。
2. **线程安全**：节点和插件必须 `Send + Sync`。
3. **性能**：渲染热路径避免不必要的分配与克隆。
4. **JS 互操作**：JS 值转换使用 `from_js` / `to_js`。
5. **跨 JS/Rust 类型**：修改后务必重新生成 bindings。
6. **注释英文**、**对话用用户偏好语言**（见本文件顶部通用规范）。
