---
applyTo: '**'
---

# Moyu Visual Novel Engine - Agent Coding Instructions

## 项目概述

末语（Moyu）是一个跨平台视觉小说引擎，采用 Rust 核心 + JavaScript/React 上层的分层架构设计。引擎支持 Windows、macOS、Linux、Android、iOS 和 Web 平台。

### 设计哲学

- **渐进式复杂度**：从简单模板到完全自定义，适应不同用户需求
- **分层架构**：Rust 底层负责性能敏感操作，JavaScript 上层提供灵活的开发体验
- **安全边界**：JS 沙盒限制文件/网络访问，Rust 层保证内存安全

## 技术栈

### 核心依赖

| 组件       | 技术                                        | 用途                       |
| ---------- | ------------------------------------------- | -------------------------- |
| 图形渲染   | wgpu + winit                                | 跨平台 WebGPU 图形后端     |
| JS 运行时  | QuickJS (quickjs-rusty)                     | 受限的 JavaScript 执行环境 |
| 音频系统   | Kira + Symphonia + Opus                     | 音频播放和解码             |
| 数学库     | glam                                        | 矩阵和向量运算             |
| 资源管理   | image, zip                                  | 图片加载和压缩包处理       |
| 异步运行时 | tokio (native) / wasm-bindgen-futures (web) | 异步任务调度               |

### 平台条件编译

```rust
// 常用的条件编译标记别名：linux， macos， android， ios， wasm， native， desktop， mobile， web
#[cfg(native)]        // 桌面和移动原生平台
#[cfg(web)]           // WebAssembly 平台
#[cfg(desktop)]       // 仅桌面平台
// 当没有配置别名时（部分项目）
#[cfg(target_os = "android")]
#[cfg(target_arch = "wasm32")]
```

## 工程结构

### Crates 概览

```
crates/
├── moyu/          # 主入口 crate，整合所有模块
├── core/          # 核心引擎：渲染循环、事件系统、节点树、插件系统
├── nodes/         # 内置节点类型：Sprite、Text、Video 等
├── runtime/       # QuickJS VM 封装和管理
├── ops/           # JS <-> Rust 桥接操作（节点创建、属性更新等）
├── resource/      # 资源加载和管理（纹理、字体等）
├── audio/         # 音频播放系统
├── platform/      # 平台抽象层（文件系统、时间、日志等）
├── scenario/      # 剧情脚本执行器
├── gamepad/       # 手柄输入支持
├── macros/        # 过程宏（Node、Plugin derive 宏等）
└── run_wasm/      # WASM 构建和开发服务器
```

### 核心 Crate 详解

#### `moyu_core` - 核心引擎

```
core/src/
├── lib.rs           # 模块导出和初始化
├── core.rs          # Core 结构体，引擎主状态
├── state.rs         # 全局状态管理
├── surface.rs       # 窗口和渲染表面创建
├── base/            # 基础类型（Transform、Point、Vertex 等）
├── core/            # 核心逻辑（渲染、事件处理）
├── events/          # 事件类型定义
├── nodes/           # 节点基类和容器
├── plugins/         # 内置插件
├── traits/          # 核心 trait 定义
└── utils/           # 工具函数
```

**关键 Traits：**

- `Node` - 场景节点 trait，所有可渲染对象必须实现
- `Renderer` - 渲染器 trait，定义如何渲染特定节点类型
- `Plugin` - 插件 trait，扩展引擎功能
- `Command` - 命令 trait，处理 JS 调用的命令
- `Focusable` - 可聚焦节点 trait

#### `moyu_nodes` - 内置节点

提供常用的节点类型：

- `Sprite` - 图片精灵，支持普通模式和九宫格模式
- `Text` - 文本渲染
- `Video` - 视频播放
- `YuvSprite` - YUV 格式视频帧渲染

#### `moyu_runtime` - JS 运行时

封装 QuickJS VM，提供：

- `QuickVM` - VM 实例管理
- `console` - 控制台 API
- `module` - 模块加载
- `ops` - 注册桥接操作
- `injections` - 全局对象注入（如 `location`），与 ops 配合使用

#### `moyu_ops` - JS 桥接

处理 JS 调用的操作：

- `create_instance` - 创建节点实例
- `destroy_instance` - 销毁节点
- `add_child` / `remove_child` - 节点树操作
- `update_props` - 更新节点属性
- `execute_node_command` - 执行节点命令
- `execute_plugin_command` - 执行插件命令

#### `moyu_platform` (moyu_pal) - 平台抽象层

提供跨平台的基础设施：

- `config` - 配置管理
- `fs` - 文件系统操作
- `dir` - 目录路径
- `logger` - 日志系统
- `time` - 时间相关
- `task` - 异步任务
- `sync` - 同步原语
- `visible_hand` - 全局状态持有器

## 开发规范

### 创建新节点

1. 在 `crates/nodes/src/nodes/` 下创建新文件
2. 使用 `#[derive(Node)]` 宏
3. 实现 `Node` trait
4. 在对应 renderer 中添加渲染逻辑

```rust
use moyu_macros::Node;
use moyu_core::nodes::NodeBase;
use moyu_core::traits::{Node, NodeBaseTrait};

#[derive(Debug, Default, Node)]
pub struct MyNode {
    // 自定义字段
    pub my_field: String,

    #[base]
    node_base: NodeBase,
}

impl Node for MyNode {
    fn node_type(&self) -> &'static str {
        "my_node"
    }

    fn update_properties(&mut self, props: &mut JSValue) {
        // 处理属性更新
    }
}
```

### 创建新插件

1. 使用 `#[derive(Plugin)]` 宏
2. 实现 `Plugin` trait
3. 在 `entry.rs` 中注册插件

```rust
use moyu_macros::Plugin;
use moyu_core::traits::{Plugin, PluginBaseTrait};

#[derive(Plugin)]
pub struct MyPlugin {
    // 插件状态
}

impl Plugin for MyPlugin {
    fn plugin_name(&self) -> &'static str {
        "my_plugin"
    }

    fn update(&mut self, vsync: bool) {
        // 每帧更新逻辑
    }
}
```

### JS 绑定

使用 `#[moyu_bindgen]` 宏暴露 Rust 函数到 JS：

```rust
#[moyu_bindgen]
fn my_function(arg: String) -> Result<String> {
    // 实现
}
```

### 资源管理

使用 `ResourceManager` 加载资源：

- 资源通过 `AssetId` 标识
- 支持自动垃圾回收（10秒扫描间隔）
- 资源路径相对于 `assets_dir()`

### 事件系统

事件类型定义在 `core/src/events/`：

- `GameEvent` - 游戏生命周期事件
- `KeyboardEvent` - 键盘事件
- `MouseEvent` - 鼠标事件
- `TouchEvent` - 触摸事件
- `NodeEvent` - 节点事件

### 错误处理

- 使用 `anyhow::Result` 进行错误传播
- 在 JS 边界使用 `Result<Option<RawJSValue>>`
- 避免 panic，优先返回错误

## 构建和测试

### 构建命令

```bash
# 开发构建
cargo build

# 发布构建
cargo build --release

# WASM 构建
cargo build --target wasm32-unknown-unknown

# Android 构建
cargo build --target aarch64-linux-android
```

### 运行示例

```bash
# 音频示例
cargo run --example audio -p moyu_audio

# 场景示例
cargo run --example simple -p moyu_runtime
```

## 注意事项

1. **平台差异**：注意 `#[cfg(...)]` 条件编译，Web 平台和 Native 平台有显著差异
2. **线程安全**：节点和插件需要实现 `Send + Sync`
3. **生命周期**：使用 `VisibleHand` 管理全局状态的生命周期
4. **性能**：渲染相关代码需要注意性能，避免不必要的分配
5. **JS 互操作**：JS 值转换使用 `from_js` 和 `to_js` 工具函数
