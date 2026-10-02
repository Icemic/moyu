# RFC：运行时输入注入

- **状态**：已接受
- **日期**：2026-10-02
- **作者**：末语项目组
- **适用范围**：`moyu_core`、`moyu_debugger`、`packages/cli`
- **相关实现**：`crates/core/src/core/pointer_events.rs`、`crates/core/src/core/keyboard_events.rs`、`crates/core/src/core/input.rs`、`crates/debugger/`、`packages/cli/src/mcp/`
- **相关 RFC**：[运行时调试桥](./2026-09-25-runtime-debug-bridge.md)（本文实现其 D7 预留的输入注入）

## 摘要

调试桥目前只有观察类能力，想触发一次点击、滚动或按键，只能在 `engine:eval` 里手工拼事件：自己找节点、算坐标、构造 `__moyu_receive_event` 的消息体。这条路径既难写（要复刻 `targetId` / `bubbleTargetIds` / `location` 的约定），又不完整（绕过命中测试、hover 状态与指针配对，富文本链接、选项按钮等依赖 `pointer_event` 的交互收不到）。

本文为调试桥增加一组**输入注入**请求：`engine:mouse`、`engine:touch`、`engine:key`，并在 MCP 层包装为 `debug_mouse`、`debug_touch`、`debug_key`。合成输入不是"伪造事件"，而是**复用引擎处理真实输入的同一条管线**：写入指针状态、命中测试、分发引擎事件、更新指针配对。因此注入一次点击的副作用与用户手点一致。

## 目标

- 支持鼠标（移动 / 按下 / 抬起 / 点击 / 滚轮）、触摸（开始 / 移动 / 结束 / 取消）、键盘（按下 / 抬起 / 按键）三类注入；
- 目标位置既可以用舞台逻辑坐标给出，也可以用节点 id（取节点包围盒中心）给出；
- 每次调用返回结构化结果：命中节点、冒泡链、实际派出的事件序列；
- native 与 web 行为一致。

## 非目标

- **可编辑文本输入（IME / 逐字输入）**：需要先解耦 `EditableTarget` 对 winit `KeyEvent` 的依赖，单独评估；当前可用节点命令 `setValue` 代替。
- **双击**：引擎当前不会从真实输入产生 `DoubleClick`，合成输入同样不产生。
- **拖拽手势的原子化封装**：连续调用 down / move / up 即可表达，不引入手势 DSL。
- **命令行镜像子命令**：`moyu debug` 暂不增加对应子命令，只走 MCP 与编辑器。

## 设计

### 合成输入归 Core

合成输入的实现放在 `moyu_core`（`crates/core/src/core/input.rs`），而不是调试桥里。理由是命中测试、指针状态（`pointer_map`）、指针配对（`down_id`）、光标与可编辑聚焦都在 Core，伪造事件绕不开这些状态。

窗口事件路径里的分发逻辑按动作拆成共享函数（`pointer_press` / `pointer_release` / `pointer_touch` / `pointer_wheel` / `dispatch_keyboard_event`），真实输入与合成输入都走它们，行为不会分叉。winit 类型（`MouseButton`、winit 的 `TouchPhase`）在窗口事件入口处映射为引擎自己的枚举，Core 对外不暴露 winit 类型。

`Core` 新增以下公开方法：

```rust
impl Core {
    pub fn simulate_pointer(&self, x: f32, y: f32, action: PointerAction) -> PointerInputReport;
    pub fn simulate_touch(&self, phase: TouchPhase, x: Option<f32>, y: Option<f32>, identifier: u32)
        -> Result<PointerInputReport, InputError>;
    pub fn simulate_key(&self, input: KeyInput);
}
```

`PointerAction` 覆盖 `Move` / `Down` / `Up(button)` / `Click(button)` / `Wheel { delta_x, delta_y, mode }`。一次 `Click` 在 Core 内部完成"移动 + 按下 + 抬起"，只做一次命中测试，避免拆成三次调用后重复派发 hover 事件。

### 线程模型

输入处理与 JavaScript 都在引擎主线程，合成输入必须在那里执行：

- native：请求线程经 `QuickVM::on_vm_thread` 投递到主线程执行，结果经 oneshot 通道回传（与 `engine:eval` 同一条路径）；
- web：请求本身就在主线程，直接执行。

这样可以保证"注入完成后随即截图/读状态"看到的是处理后的结果。

### 坐标与目标

- 坐标统一为**舞台逻辑坐标**，与 `engine:props` 的 `bounds` 同一坐标系；
- `nodeId` 与 `x`/`y` 二选一，同时给出或都不给出报错；
- 用 `nodeId` 时取节点 `globalContentBounds` 的中心；包围盒为空时回退到节点全局变换后的局部原点；
- 包围盒来自最近一次渲染，节点刚创建、尚未渲染时可能不准。

注入不保证一定命中请求的节点：实际命中由命中测试决定，结果里如实报告目标节点；不做"自动寻找可命中点"的容错。

### 会话语义

- 除 `move` 外的动作先把指针"瞬移"到目标点并刷新 hover，因此单发一次 `click` 即可命中；
- `click` = 移动 + 按下 + 抬起；抬起与按下配对成功时才产生 `Click`（右键为 `ContextMenu`），与真实输入一致；
- 触摸是有状态的会话：`start` 必须先于 `move` / `end` / `cancel`，且只接受 `start` 时已存在的标识符；`move` / `end` / `cancel` 可以省略坐标（沿用上次位置）；
- 键盘事件与真实路径一致：目标为全局（`targetId` 为 0），修饰键按调用方传入的值填充，不改引擎的全局修饰键状态。

### 结果

指针类动作返回 `PointerInputReport`：

```json
{
  "action": "click",
  "point": { "x": 640.0, "y": 360.0 },
  "targetNodeId": 42,
  "bubbleNodeIds": [3, 0],
  "dispatched": ["MouseMove", "MouseEnter", "MouseDown", "MouseUp", "Click"]
}
```

`dispatched` 里是引擎实际派出的事件种类，名字与 JavaScript 侧看到的一致（PascalCase，如 `MouseDown`）。未与按下配对的抬起只派发 `MouseUp`，不会有 `Click`，调用方可以从这个列表里看出差异。

## 协议

三个请求都复用请求/响应信封，参数为 camelCase：

### `engine:mouse`

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `action` | `move` / `down` / `up` / `click` / `wheel` | 必填 |
| `x`、`y` | number | 舞台逻辑坐标，与 `nodeId` 二选一 |
| `nodeId` | number | 目标节点，取包围盒中心 |
| `button` | `left` / `right` / `middle` | 默认 `left` |
| `deltaX`、`deltaY` | number | 仅 `wheel`，默认 0 |
| `mode` | `line` / `pixel` | 仅 `wheel`，默认 `line` |

### `engine:touch`

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `action` | `start` / `move` / `end` / `cancel` | 必填 |
| `x`、`y` | number | 与 `nodeId` 二选一；`start` 必填，其余可省略 |
| `nodeId` | number | 目标节点，取包围盒中心 |
| `identifier` | number | 触摸点标识，默认 0 |

### `engine:key`

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `action` | `down` / `up` / `press` | 必填 |
| `key` | string | 必填，JavaScript `event.key` 的值，如 `Escape`、`Enter`、`a` |
| `code` | string | 物理键名，缺省与 `key` 相同 |
| `repeat` | boolean | 默认 false |
| `ctrlKey`、`shiftKey`、`altKey`、`metaKey` | boolean | 默认 false |

响应为 `engine:mouse:done` / `engine:touch:done` / `engine:key:done`；指针类附带上述结果字段，键盘类回显 `action` / `key` / `code` 与 `dispatched`（如 `["KeyDown"]`）。

## MCP 工具

| 工具 | 说明 |
| --- | --- |
| `debug_mouse` | 鼠标动作；参数与 `engine:mouse` 一致 |
| `debug_touch` | 触摸动作；参数与 `engine:touch` 一致 |
| `debug_key` | 键盘动作；参数与 `engine:key` 一致 |

`engine:hello` 的 `capabilities` 增加 `mouse`、`touch`、`key`。

## 决策记录

| 编号 | 决策 | 日期 |
| --- | --- | --- |
| D1 | 合成输入实现在 `moyu_core` 而非调试桥，复用真实输入的分发管线；窗口事件路径按动作拆出共享函数，winit 类型在入口处映射。 | 2026-10-02 |
| D2 | native 经 `on_vm_thread` 在主线程执行合成输入并回传结果，web 直接执行；保证"注入后立刻观察"的时序。 | 2026-10-02 |
| D3 | 坐标语义为舞台逻辑坐标；目标用 `x`/`y` 或 `nodeId` 二选一，`nodeId` 取 `globalContentBounds` 中心（空包围盒回退到变换后的原点）。 | 2026-10-02 |
| D4 | 除 `move` 外的动作先瞬移指针并刷新 hover；`click` 在 Core 内完成移动 + 按下 + 抬起，只做一次命中测试。 | 2026-10-02 |
| D5 | 触摸为有状态会话：`start` 必须先于其他阶段，`move` / `end` / `cancel` 可省略坐标；对未开始的标识符报错而不自动创建。 | 2026-10-02 |
| D6 | 每次调用返回命中节点、冒泡链与实际派出的事件序列；不保证命中请求的节点，也不做自动寻找可命中点。 | 2026-10-02 |
| D7 | 不实现可编辑文本注入（IME / 逐字输入）与双击；前者需先解耦 `EditableTarget` 与 winit `KeyEvent`，后者引擎不从真实输入产生。 | 2026-10-02 |
| D8 | 键盘事件与真实路径一致：全局目标、修饰键按参数填充、不改全局修饰键状态；v1 不提供命令行镜像子命令。 | 2026-10-02 |

## 分阶段实施

### M1：Core 合成输入（0.5 天）

- `crates/core/src/core/input.rs`：公开类型与 `simulate_*` 方法；
- `pointer_events.rs` / `keyboard_events.rs`：按动作拆出共享分发函数，真实路径改用它们。

验收：`cargo build` 通过；真实输入行为不变（人工核对）。

### M2：调试桥与 MCP（0.5 天）

- `engine:mouse` / `engine:touch` / `engine:key` 请求与应答；
- native / web 的"主线程执行"辅助；
- CLI 的 `debug_mouse` / `debug_touch` / `debug_key` 工具。

验收：MCP 工具可以点击 framework 标题页按钮、推进对话、滚动打开 backlog。

## 影响面与风险

- **性能**：共享函数在真实输入路径上多一次可空的事件记录（`DispatchRecord`），未启用记录时无分配；主线程往返只发生在合成输入调用上。
- **一致性**：合成输入与真实输入共用分发函数，两者不会产生行为分叉；窗口事件入口的 winit 映射是唯一新增的手工转换点。
- **安全**：输入注入与 `engine:eval` 同级，属于控制类能力，同样只在调试端点注入时启用、只监听 loopback。
- **范围蔓延**：本文只覆盖鼠标、触摸、键盘三类原语；手势、录制回放、可编辑文本输入不在范围内，必要时另开 RFC。
