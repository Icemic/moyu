# RFC：无窗口运行（headless）

- **状态**：已实现
- **日期**：2026-10-22
- **作者**：末语项目组
- **适用范围**：`moyu`（默认 runtime）、`moyu_app`、`moyu_pal`、`moyu_debugger`、`packages/cli`
- **相关实现**：`crates/moyu/src/headless/mod.rs`、`crates/moyu/src/headless/window_system.rs`、`crates/moyu/src/main.rs`、`crates/moyu/src/entry.rs`、`crates/app/src/lib.rs`、`crates/platform/src/platform.rs`、`crates/debugger/src/state.rs`、`crates/core/src/core/render_output.rs`、`crates/core/src/surface.rs`、`packages/cli/src/utils/debug-session.ts`、`packages/cli/src/commands/debug.ts`、`packages/cli/src/mcp/tools.ts`
- **相关 RFC**：[窗口系统抽象](./2026-09-06-window-system-abstraction.md)、[运行时调试桥](./2026-09-25-runtime-debug-bridge.md)

## 摘要

引擎已经能渲染到自建纹理：窗口系统的 `create_surface` 返回 `Unsupported` 时，`create_render_output` 改用 `TextureOutput`，画面留在引擎持有的纹理里，截图与调试桥读的就是它。Godot 嵌入走的是 `Core::init_graphics_on_device`，它直接构造 `TextureOutput`，不经过这条分支；因此「`create_surface` → `Unsupported` → `TextureOutput`」此前没有生产调用方，headless 是首次启用。

本文给默认 runtime 增加 `--headless`：按窗口模式同样的顺序装配核心、调试桥、节点、插件与项目脚本，但窗口系统换成无窗口实现，帧循环由 runtime 自己按 60 Hz 驱动。窗口不再出现，画面、截图与调试桥行为不变。

命令行与 MCP 的调试会话默认使用这个模式，因此 `debug_start` 不再把游戏窗口盖在用户的桌面上。

## 背景

### 现状

- 默认 runtime（`crates/moyu`）只有一条启动路径：`main_entry` 建 winit 事件循环，`resumed` 里建窗口，`finish_start` 里 `set_visible(true)`。`moyu debug` 与 `moyu mcp` 启动的都是它，所以每次调试都会弹出游戏窗口。
- 窗口系统抽象（RFC 2026-09-06）把窗口、事件循环与 surface 从核心移出，`create_surface` 返回 `Unsupported` 表示「本窗口系统没有 surface」。渲染输出边界（提交 `0475166`）据此让引擎渲染到自建纹理，纹理格式为 `Rgba8Unorm`，带 `TEXTURE_BINDING | COPY_SRC`，因此既能被采样，也能截图。
- 引擎自身的图形线程（`Core::init_graphics` 在 native 上创建 "Graphic Update" 与 "Graphic Render" 两个线程）与窗口无关：`WindowSystemEvent::Frame` 唤醒更新线程，渲染线程从命令队列取帧。只要有人按帧节奏派发事件，画面就会持续产出。
- 调试桥只依赖 Core 存在与调试端点：`moyu_debugger::start()` 在 `set_core` 之后调用，截图走 `Graphics::request_snapshot` + 渲染时回读，`engine:eval` 投递到 VM 线程。这些都与窗口无关。

### 现状不足

调试一个正在开发的项目时，游戏窗口会抢焦点、占屏幕，干扰同一台机器上的其他工作。web 会话避开了这个问题，但需要浏览器、且与 native 的运行时行为有差异（平台、字体、资源读取路径都不同），不适合作为 native 问题的替代验证手段。

## 目标

- 提供 `--headless`，让默认 runtime 在没有任何窗口的情况下运行完整项目；
- 引擎行为与窗口模式一致：节点树、属性、日志、求值、输入注入、截图全部可用；
- 调试命令行与 MCP 的 native 会话默认使用该模式，需要看画面时能显式关闭；
- 能力可观测：客户端能确认当前会话没有窗口。

## 非目标

- **web 的 headless**：页面本身就是运行环境，浏览器标签页不需要额外模式。
- **移动端**：`--headless` 只用于 `packages/cli` 能拉起的桌面 native 引擎。
- **无 GPU 环境**：仍需可用的 wgpu 适配器。没有显示器的 Linux 服务器通常可以用 Vulkan 或软件适配器；本文不引入纯 CPU 渲染。
- **帧率模拟与时间控制**：固定 60 Hz，不模拟可变刷新率，不提供暂停单步。
- **`moyu run --headless`**：`run` 是给人手动运行项目用的，窗口就是它的目的；本文只让调试会话默认无窗口。

## 设计

### 开关

`--headless` 是引擎可执行文件的命令行开关，由 `crates/moyu/src/main.rs` 在解析项目配置之前检查：

```rust
if std::env::args().any(|arg| arg == "--headless") {
    headless::run().await;
    return;
}
```

放在 `config::setup()` 之前：无窗口入口要先 `disable_error_dialogs()`，坏配置才不会卡在一个没人能关掉的模态对话框上。

`moyu_pal::config::setup()` 用 `pico_args` 只取 `--entry` 与 `--params`，未消费的参数不影响解析，因此该开关不需要进入 `MoyuConfig`。窗口模式与调试桥都保持原样。

不做 `--no-headless` 之类的引擎侧反向开关：不传即窗口模式。反向开关只出现在 CLI（`--no-headless`），因为它默认开启。

### 装配顺序

与 `entry.rs` 的 `WinitApplication` 保持同序，差别只在于窗口系统的来源和帧循环的驱动者：

1. `platform::disable_error_dialogs()` 与 `platform::set_headless(true)`；
2. `platform::setup()`、`moyu_core::setup()`；
3. `config::setup().await`（读 `--entry` / `--params`）；
4. `Core::new(HeadlessWindowSystem)` + `set_core`；
5. `moyu_debugger::start()`；
6. `moyu_app::prepare(&core).await`（图形 → 内置节点 → 插件）；
7. `sync_surface_size_with_window(false)`；
8. `load_user_script`；
9. `announce_ready(core, false)`（跳过启动画面）；
10. 同步帧循环。

第 4～9 步与窗口模式逐项对应，共用 `moyu_app::prepare` 与 `load_user_script`，两边只在三处不同：

| 项 | 窗口模式 | 无窗口 |
| --- | --- | --- |
| 调 `prepare` | `block_on_without_runtime(prepare(&core))` | `prepare(&core).await` |
| 窗口动作 | `move_to_center` + `set_visible` + `request_redraw` | 不调用 |
| `announce_ready` | 传 `true` | 传 `false` |

Steam 不用处理：`main.rs` 只在窗口分支初始化 `SteamPlugin`，无窗口分支走不到那段代码，`prepare` 也不碰 Steam（它由嵌入方注入）。

### 帧循环

```rust
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

while !window_system.quit_requested() {
    let started = Instant::now();
    core.handle_window_system_event(WindowSystemEvent::Idle);
    core.handle_window_system_event(WindowSystemEvent::Frame);
    std::thread::sleep(FRAME_INTERVAL.saturating_sub(started.elapsed()));
}
```

- `Frame` 唤醒引擎的更新线程并触发 `vm.tick()`，渲染由渲染线程完成，因此循环不调用 `render_frame_inline`（它属于调用方自带帧循环的嵌入场景）。
- 不安装 VM 唤醒钩子。窗口模式把 `vm.tick()` 投成事件，无窗口没有事件循环，`on_vm_thread` 推进的任务落到下一帧（≤16 ms）执行；`engine:eval` 与 `setTimeout` 的延迟都在帧粒度内。
- 循环在创建 VM 之后不再 `await`。脚本运行时的线程身份记录在线程局部变量里（`QuickVM::is_vm_thread`），跨 worker 线程恢复会破坏它，所以装配期间的所有 `await` 必须排在 `load_user_script` 之前，之后一律同步。
- 固定 16 ms 间隔；不做漂移校正之外的补偿（当前实现即「按用时补齐」，与窗口模式首次排帧的做法相同）。

退出：`core.quit()`（`system.quit` 命令）走 `WindowSystem::request_quit`，无窗口实现只置一个标志，循环看到后退出。

### 无窗口语义

| 能力 | 无窗口实现 | 理由 |
| --- | --- | --- |
| `create_surface` | `Unsupported` | 引擎改用纹理输出 |
| `surface_metrics` | `initial_surface_size`，缩放比 1 | 与窗口模式的初始尺寸一致，截图尺寸可预期 |
| `has_focus` | `false` | 没有窗口可聚焦；同时让 `EditableManager` 不去申请 IME |
| `window_state` | `Idle` | 没有最大化 / 最小化 / 全屏 |
| `request_redraw` | 空 | 帧由循环驱动 |
| `request_quit` | 置退出标志 | 循环的唯一退出条件 |
| `set_title` | `Ok` | 项目脚本常调，静默接受 |
| `set_window_state` / `request_window_size` / `move_to_center` / IME | `Unsupported` | 与移动端、Godot 的处理一致：不支持的窗口操作报错，由调用方决定是否记录 |

不派发 `Focused` 事件。框架在 `blur` 时停止 auto，无窗口会话没有「窗口失去焦点」这回事，auto 应当继续运行。

### 启动画面与致命错误

- **启动画面**：`announce_ready` 增加是否显示启动画面的参数。无窗口时传 `false`，否则启动画面会覆盖前 2 秒的画面，而调试会话正好在这段时间截图。
- **致命错误**：`show_fatal_error_and_exit` 在桌面平台会弹模态对话框。无窗口时没有人能点掉它，进程会挂住；因此 `moyu_pal::platform` 增加 `disable_error_dialogs()`，无窗口入口在启动时调用，失败改为写日志后退出。退出码仍为 1。

### 调试桥与截图

无需改动：调试桥在 `set_core` 之后启动，截图的快照路径对 `TextureOutput` 已支持（`supports_copy_src` 为真）。`engine:state` 增加一个 `headless` 字段，由启动参数置位的全局标志决定，供客户端确认会话形态：

- 安装了旧版引擎时，`--headless` 会被忽略，窗口照常打开；此时客户端看到 `headless: false`，据此可以判断会话形态。
- 该字段描述的是「以无窗口方式启动」，与帧落到哪里无关：Godot 的帧也在自建纹理里，但它不是 headless。

### 音频

无窗口不改变音频路径：`AudioManager::new()` 照常初始化，项目播放的 BGM / 音效会从默认输出设备发声。这既是「行为与窗口模式一致」的要求，也会在用户做别的事情时出声。取舍见「已定结论」。

### 命令行与 MCP

- `SessionOptions` 增加 `headless?: boolean`，native 会话缺省为 `true`，启动时追加 `--headless`；web 与 attach 会话忽略该值。
- `moyu debug <子命令>` 的所有会话参数增加 `headless`（布尔，默认 `true`）；`citty` 支持 `--no-` 前缀，因此需要看画面时用 `--no-headless`。
- MCP 的 `debug_start` 增加 `headless`（默认 `true`），输出里标明会话是否无窗口。
- 会话结果带上 `headless`，供调用方判断。

`citty` 把布尔值交给 Node 的 `parseArgs`，所以 `--headless=false` 解析出来是字符串 `"false"` 而非布尔 `false`。`runSession` 同时读两种形态（`!== false && !== 'false'`），不依赖用户只写一种语法。

## 影响面与风险

- **引擎侧接触点**：新增一个窗口系统实现、一个启动入口，`announce_ready` 多一个参数，`moyu_pal::platform` 多一个开关。核心、节点、插件、调试桥均不新增状态。
- **安装的引擎版本**：旧版引擎不认识 `--headless`，会退回窗口模式。`engine:state.headless` 让这种不匹配可观测。
- **GPU 依赖**：无窗口不等于无图形。无适配器的环境仍然启动失败，且此时没有对话框，只能从日志看原因。
- **音频**：见上。
- **性能**：多一个每帧派发事件的循环，开销与 winit 事件循环同量级；纹理输出比 swapchain 多一次纹理分配（固定尺寸，不随窗口变化）。

## 替代方案

- **隐藏窗口**：创建窗口但不显示。改动最小，但 winit 仍会接管事件循环与输入法，且在部分平台上隐藏窗口仍会计入任务栏 / 窗口切换；并不能达到「不干扰」的目的。
- **复用自建帧循环的嵌入示例**：这类示例是自足的演示，缺少调试桥、插件、脚本与帧节奏；把它扩展成运行时会变成第二套启动路径。
- **只让 MCP 用 web 模式**：web 与 native 的平台差异（字体、资源读取、wasm 边界）会让 native 问题难以复现，且需要额外的浏览器进程。

## 已定结论

1. **音频保持出声**。与窗口模式一致，不引入 `--mute`。无窗口会话应当和窗口会话表现一致，静音会让音频相关的缺陷不易察觉。
2. **旧版引擎不做前置校验**。CLI 不比对最低版本、不拒绝启动，只在 `engine:state.headless` 里如实报告。

## 实现与验证

### 已实测通过的能力

用 `target/debug/moyu --headless` 配合调试桥（`attach` 模式）实测：

| 能力 | 结果 |
| --- | --- |
| 无窗口启动 | 进程运行，桌面无窗口 |
| `engine:state` | `headless: true`，`surfaceSize` 等于配置的 `initialSurfaceSize` |
| 首帧与 `engine:ready` | 正常到达，节点数随项目内容增长 |
| 截图 | 出图，尺寸等于 stage 按 16:9 落在 surface 内的区域；内容与背景色一致 |
| `eval` | 读写 `globalThis` 正常 |
| 输入注入 | 鼠标悬停 / 点击派发 `MouseEnter`、`Click`，节点响应并切换样式 |
| 完整应用 | `packages/gallery` 在无窗口下渲染 React 应用，页面切换、资源加载与 GC 均正常 |
| 退出 | `system.quit` 后进程退出，退出码 0 |
| 致命错误 | 坏 `--entry` 立即退出（码 1），不弹对话框 |
| 帧节奏 | 空闲时 5 秒只消耗 0.3 秒 CPU，循环在 sleep 而非忙等 |

### 已知限制

- **`--headless` 只用于桌面**。`crates/moyu/src/lib.rs` 的 `android_main` 与 `moyu_init` 不做分流。
- **本地 file entry 依赖 `node_resolve`**：不含路径的模块名（如 `index.js`）需要 `node_modules` 才能解析。这是既有行为，与无窗口无关；用 http entry 可绕过。
- **截图尺寸小于 surface**：`surface_metrics` 报告 `initial_surface_size`，但截图只覆盖 stage 实际占用的区域，比例不符时四周是背景色。这是渲染管线的既有行为。
- **无 GPU 时启动失败**：`create_render_context` 找不到适配器会 panic，无窗口下只能从日志看到原因（`disable_error_dialogs` 让对话框不出现，但这里走的是 panic 而非 `show_fatal_error_and_exit`）。
- **帧率固定 60 Hz**：高刷新率显示器上动画观感与窗口模式不同。
