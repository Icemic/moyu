# RFC：鼠标按键语义与 DOM 对齐

- **状态**：已接受
- **日期**：2026-10-02
- **作者**：末语项目组
- **适用范围**：`moyu_core`、`moyu_debugger`、`packages/kit`、`packages/cli`、`moyu-docs`
- **相关实现**：`crates/core/src/events/mouse.rs`、`crates/core/src/core/pointer_events.rs`、`crates/core/src/core/input.rs`、`crates/core/src/core/keyboard_events.rs`、`crates/core/src/state.rs`、`crates/debugger/`、`packages/kit/src/events/`、`packages/kit/src/declaration.ts`、`packages/cli/src/mcp/`
- **相关 RFC**：[运行时输入注入](./2026-10-02-input-injection.md)（本文扩展其 `engine:mouse` 的 `button` 语义，并取代其 D7 中"引擎不产生双击"的结论）

## 摘要

事件系统以 DOM 事件为蓝本（`clientX` / `screenX` / `offsetX` 命名、冒泡、`preventDefault`、事件 kind 命名），官方文档（`moyu-docs` 的 `engine-api/events.md`）也已把 `button` 列为 `MouseEvent` 属性。但实现从未携带该字段：JS 侧无法区分左右中键与侧键，`MouseEventKind::DoubleClick` 定义了却没有任何派发点，文档列出的 `x` / `y` 与修饰键字段同样不存在。

本文把鼠标事件的按键语义补齐到 DOM 语义，并顺带修正配对与目标规则：

1. 所有鼠标事件携带 `button` / `buttons`（DOM 数字编码 0-4 与位掩码；DOM 未定义的其他物理键不产生事件）；
2. 配对从"单槽 + 同节点"改为"按键级 + 最近公共祖先"，修复多键互相覆盖与跨节点抬手丢 `Click`；
3. 补 `AuxClick`（所有非主键）与 `DoubleClick`（左键双击）；
4. `ContextMenu` 从「抬起且配对后」改为右键按下即派发（Chromium/Linux 实测行为）；
5. 补齐修饰键字段（引擎已有 `modifiers_state`，此前只有键盘事件在用）；
6. kit 层补 `x` / `y` 别名、监听 props 与按键常量；
7. 调试桥可注入并验证以上全部。

字段取值与事件顺序以 Chromium 实测为准（见附录）。规范留给平台/实现的细节取实测值；DOM 未定义或本次不做的场景一律明确列为「不支持」，不做近似实现。

## 背景

现状（改动前）：

- 窗口事件入口 `WindowEvent::MouseInput` 的 `Pressed` 分支不把 winit 的 `button` 传下去，`pointer_press` 根本不知道按了哪个键；只有 `Released` 分支映射为 `PointerButton`。
- 按键只间接体现在 kind 上：抬起与按下配对到同一节点时，左键额外产生 `Click`、右键额外产生 `ContextMenu`，中键 / 后退 / 前进无任何手势事件。
- `PointerState.down_id` 是单槽 `Option<u32>`：多键同时按下会互相覆盖配对记录；按下与抬起不在同一节点时完全不派发 `Click`（DOM 语义是派发到最近公共祖先）。
- `MouseEventKind::DoubleClick` 全仓库无派发点；`MoyuListenerAttributes` 也没有 `onDoubleClick` / `onContextMenu` 声明。
- 文档承诺的 `x` / `y` 与 `ctrlKey` 等修饰键字段缺失。修饰键状态引擎已有（`Core::modifiers_state`，由 `WindowEvent::ModifiersChanged` 维护），仅键盘事件在使用。

对照基准：DOM 规范对若干细节本身即为平台/实现定义（`contextmenu` 时机、双击判定阈值、事件顺序），因此字段取值与事件顺序以 Chromium（Linux）实测为准，实测记录见附录。规范与实测冲突之处，以规范语义为准并单独说明。

## 目标

- 所有鼠标事件携带 DOM 语义的 `button` / `buttons`：`button` 取 0-4（左 / 中 / 右 / 后退 / 前进），`buttons` 为位掩码；DOM 未定义的其他物理键不产生鼠标事件。
- 配对改为按键级 + 最近公共祖先（DOM 语义），修复多键覆盖与跨节点丢 `Click`。
- 补 `AuxClick`（所有非主键）与 `DoubleClick`（左键）；`ContextMenu` 时机对齐 Chromium/Linux（按下即派发）；修饰键随事件派发。
- kit 补 `x` / `y` 别名、`onAuxClick` / `onContextMenu` / `onDoubleClick` props 与按键常量，重新生成绑定。
- 调试桥的 `down` 携带按键，`button` 支持 `back` / `forward`，注入键盘同步修饰键状态，使以上能力都能经 MCP 验证。

## 非目标

- **其他物理按键（`MouseButton::Other`）不支持**：DOM 未定义 0-4 之外的 `button` 值；引擎选择不实现——这些键不产生鼠标事件，也不占 `buttons` 位。宁可没有，不做近似。
- **DOM `detail`（点击计数）与 `which` 不实现**：不在引擎承诺的字段内；事件不携带该字段。
- **`WheelEvent` 不携带坐标**：轮盘事件当前连 `clientX` 等位置字段都没有，是独立缺口，另案。
- **触摸事件不加按键字段**：Web 的 Touch 事件同样没有 `button` / `buttons`。
- **节点级 `PointerEventKind` 不加按键信息**：富文本链接等场景不区分按键，维持现状。
- **`MouseEnter` / `MouseLeave` 的语义与冒泡本次不修正（记录在案）**：实测 DOM 的 `mouseenter` / `mouseleave` 不冒泡，且仅在进入 / 离开元素边界时触发一次；引擎现有事件是「每次命中目标变化触发 + 冒泡」，实为 `mouseover` / `mouseout` 语义。修正需要引入 hover 链差分与新事件种类，会破坏 kit 现有组件（如 Button 的 hover 判断），另案处理；本次仅订正文档描述。
- **Windows 的 `contextmenu` 时机不模拟**：引擎取 Chromium/Linux 的按下时机并文档化；Windows 的抬起时机属平台差异，不引入运行时平台分支。
- **不实现键盘触发的语境菜单**（Menu 键）。

## 设计

### 1. 数值编码（DOM 对齐）

`button`（事件的触发键）：

| 值 | 键 | 说明 |
| --- | --- | --- |
| 0 | 左 / 主键 | |
| 1 | 中键 | DOM 的坑：`button = 1`，但 `buttons` 位是 4 |
| 2 | 右 / 次键 | |
| 3 | 后退（X1） | |
| 4 | 前进（X2） | |

0-4 之外没有值：其他物理键（winit 的 `MouseButton::Other`）不产生鼠标事件（见非目标）。

`buttons`（事件发生时按住的键的位掩码）：`1` 左、`2` 右、`4` 中、`8` 后退、`16` 前进。

各 kind 携带的值（Chromium 实测对照）：

| kind | button | buttons |
| --- | --- | --- |
| MouseDown | 触发键 | 按下后包含该键（实测左 1 / 右 2 / 中 4） |
| MouseUp | 触发键 | 抬起后不含该键 |
| Click / DoubleClick | 0 | 0 |
| AuxClick | 触发键（实测中键 1 / 右键 2） | 0 |
| ContextMenu | 2 | 按下后的掩码（实测仍含右键位 2） |
| MouseMove / MouseEnter / MouseLeave | 0 | 当前按住掩码（实测拖动中为按住值） |

事件顺序（Chromium/Linux 实测，引擎对齐）：

| 手势 | 序列 |
| --- | --- |
| 左键单击 | `MouseDown → MouseUp → Click` |
| 右键单击 | `MouseDown → ContextMenu → MouseUp → AuxClick` |
| 中键单击 | `MouseDown → MouseUp → AuxClick` |
| 左键双击 | `MouseDown → MouseUp → Click` ×2 → `DoubleClick` |
| 跨元素抬起（配对成功） | 手势事件（`Click` / `AuxClick` / `DoubleClick`）的 target 为按下与抬起目标的最近公共祖先 |

`ContextMenu` 在按下即派发是 Chromium/Linux 行为（Windows 为抬起后，属平台差异，引擎不模拟）；`AuxClick` 覆盖所有非主键（中 / 右 / 后退 / 前进）。

### 2. Rust 类型与状态

`crates/core/src/events/mouse.rs`：

- `MouseEventKind` 新增 `AuxClick`（`Click` 之后）。
- `MouseEvent` 新增字段：`button: i32`、`buttons: u32`、`ctrl_key: bool`、`shift_key: bool`、`alt_key: bool`、`meta_key: bool`。修饰键命名与 `KeyboardEvent` 一致（camelCase 序列化后为 `ctrlKey` 等）。

`crates/core/src/core/input.rs`：

- `PointerButton` 只保留 DOM 定义的 5 个键：`Left` / `Middle` / `Right` / `Back` / `Forward`（删除 `Other`）。
- `PointerButton::as_button_number(self) -> i32`（0 / 1 / 2 / 3 / 4）；`bit(self) -> u32`（1 / 4 / 2 / 8 / 16）。
- `PointerAction::Down` 变为 `Down(PointerButton)`。
- 窗口入口把 winit `MouseButton` 映射为 `Option<PointerButton>`，`MouseButton::Other(_)` 返回 `None` 并被忽略，不进入事件管线。

`crates/core/src/state.rs`（`PointerState`）：

- `down_id: Option<u32>` 更名为 `touch_down_id`，只服务触摸会话（`start` 置位、`end` / `cancel` 清除）。
- 新增 `mouse_downs: [Option<MousePress>; 5]`（按 Left / Middle / Right / Back / Forward 索引），`MousePress { node_id, parent_ids }` 实现按键级配对；`buttons` 掩码由它推导，保持单一事实来源。
- 新增 `last_click: Option<ClickRecord>`，供 DoubleClick 判定。

修饰键读取已有的 `Core::modifiers_state`（`ArcSwap<ModifiersState>`）。

### 3. 派发规则

按下 `pointer_press(identifier, button)`：

- 派发 `MouseDown`（button = 按键编号，buttons = 按下后的掩码）；
- 记录 `mouse_downs[button] = 当前 hover 目标`（含祖先链）；
- 若为右键，紧接着派发 `ContextMenu`（button = 2，buttons 仍含右键位，target = 按下的 hover 目标）——对齐 Chromium/Linux 的按下时机，与抬起和配对无关。

抬起 `pointer_release(identifier, button)`：

- 派发 `MouseUp`（button = 按键编号，buttons 已排除该键）；
- 若该键在 `mouse_downs` 中有记录：计算按下目标与抬起目标的**最近公共祖先** C（祖先链为 root→…→node，取两链最长公共前缀的末项；没有公共项时 C = 根节点 0），按按键派发手势事件：
  - **左键**：`Click`（target = C）；随后做双击判定——与上次左键点击同为 C、间隔 ≤ 500ms、位移 ≤ 4px 时追加 `DoubleClick`（同 target = C）并重置记录，否则更新记录。
  - **中键 / 右键 / 后退 / 前进**：`AuxClick`（target = C）。右键的 `ContextMenu` 已在按下时派发，此处不再派发。
- 清除该键的记录；未配对的抬起只派发 `MouseUp`（保持现状）。

固定序列（Chromium 实测对照）：左键 `MouseDown → MouseUp → Click`；右键 `MouseDown → ContextMenu → MouseUp → AuxClick`；中键 `MouseDown → MouseUp → AuxClick`。

说明：

- 手势目标为根节点 0 时（两链无公共祖先，或按下 / 抬起都落在非交互区域），kit 现有实现跳过节点监听、只触发全局监听；与 DOM 中 click 落在 document 上一致。
- `DoubleClick` 只由左键产生（DOM 行为）；判定阈值（500ms / 4px）在规范与实测中都属实现定义（MDN：`detail` 重置间隔 may vary from browser to browser and across platforms），引擎取固定值并文档化。
- 节点级 focusable 事件（富文本链接）是引擎自有抽象，不是 DOM 映射，维持既有语义：`Down` / `Up` 发给 hover 目标；`Click` 仅在按下与抬起命中同一节点时派发（既有条件）。DOM 层的 `Click` / `AuxClick` / `DoubleClick` 走上面的公共祖先规则，两者互不影响。

### 4. 修饰键

- 所有鼠标事件读取 `Core::modifiers_state`，填充 `ctrlKey` / `shiftKey` / `altKey` / `metaKey`。
- `Core::simulate_key` 增加一步：把注入的 `KeyboardModifiers` 写入 `modifiers_state`，使合成点击能携带修饰键。真实路径不变，仍由 `ModifiersChanged` 维护。

### 5. 调试桥（moyu_debugger / packages/cli）

- `PointerAction::Down(button)`；`MouseRequest.button` 增加 `back` / `forward`，作用于 `down` / `up` / `click`。
- MCP `debug_mouse` 的 `button` 枚举同步扩展，描述改为 "down、up、click 使用"。

### 6. kit（packages/kit）

- 重新生成 bindings：`RawMouseEvent` 增加字段、`MouseEventKind` 增加 `'AuxClick'`。
- `events/mouse.ts`：`MouseEvent` 接口补 `button` / `buttons` / `x` / `y` / `ctrlKey` / `shiftKey` / `altKey` / `metaKey`；`x` / `y` 在事件包装时赋值为 `clientX` / `clientY`（DOM 中二者本就是别名，不进入引擎载荷重复传输）。
- `declaration.ts`：`MoyuListenerAttributes` 增加 `onAuxClick` / `onContextMenu` / `onDoubleClick`。
- 新增 `MouseButton` 常量（`Left: 0, Middle: 1, Right: 2, Back: 3, Forward: 4`，不含未知键）与 `MouseButtons` 位掩码常量（`Left: 1, Right: 2, Middle: 4, Back: 8, Forward: 16`）。

### 7. 文档（moyu-docs）

`engine-api/events.md`：

- MouseEvent 属性表补全（`button` / `buttons` / `x` / `y` / 修饰键），鼠标事件表补 `onAuxClick` / `onContextMenu` / `onDoubleClick`，全局事件列表补 `auxclick`；
- 增加事件顺序表（§1）与配对 / 公共祖先规则；
- 订正现有错误描述：`onMouseEnter` / `onMouseLeave` 现写作「不冒泡」，与实现不符——引擎的这两个事件每次命中目标变化都会触发并冒泡（`mouseover` / `mouseout` 语义），文档如实描述并注明与 DOM 的差异；
- 增加「与 DOM 的已知差异」小节：其他物理按键不支持、`detail` 未实现、`WheelEvent` 无坐标、Enter / Leave 语义差异。

## 兼容性

- 事件载荷纯增字段，旧消费方忽略即无感；事件 kind 只增 `AuxClick`。
- 行为变化一：跨节点按下 / 抬起现在会派发手势事件（目标为最近公共祖先），此前完全不派发。
- 行为变化二：`ContextMenu` 从「抬起且配对后」改到「右键按下时」（对齐 Chromium/Linux 实测）。
- 行为变化三：其他物理键（`Other`）从「派发 `MouseDown` / `MouseUp` 但无手势」改为「完全不派发」。
- 已核实下游无依赖：`moyu-framework` 未使用 `contextmenu` / `auxclick` / `dblclick`（grep 零命中），kit 未声明相关 props；fishflow 的 ContextMenu 全部是 React DOM 组件，与引擎事件无关。kit 的 Button 用 `onMouseUp` + 内部 pressed 状态，不依赖 Click 配对。
- `PointerState.down_id` 更名只影响 crate 内部；`PointerAction::Down` 的形状变化只影响 debugger 一个调用点。
- 无数据模型 / 存档影响。

## 验证

- `cargo build`；`yarn generate:bindings`；kit 类型检查。
- MCP 实测（断言按 Chromium 实测序列）：
  - `debug_mouse` 右键 `click` → `dispatched` 为 `["MouseMove","MouseEnter","MouseDown","ContextMenu","MouseUp","AuxClick"]`；
  - 中键 `click` → 以 `MouseDown`、`MouseUp`、`AuxClick` 结尾且无 `ContextMenu`；
  - 左键 `click` → 以 `MouseUp`、`Click` 结尾；
  - `debug_key` 注入 ctrl 按下 + `debug_mouse` `click` → 全局监听读到 `ctrlKey: true`，`buttons` 值正确；
  - 两次快速左键 → 第二次点击额外派出 `DoubleClick`；间隔 > 500ms → 无；
  - 跨元素：`down` 于节点 A、`up` 于节点 B（A 为 B 祖先）→ 手势事件 target 为最近公共祖先。

### 实测记录（gallery，本地构建引擎）

在 `packages/gallery` 上以本地构建的引擎（`cargo run --release`，attach 到调试桥）执行，用包裹 `__moyu_receive_event` 的方式读取引擎实际送出的载荷：

| 场景 | 实测载荷 |
| --- | --- |
| 右键 `click` | `MouseDown(2, btns=2)` → `ContextMenu(2, btns=2)` → `MouseUp(2, btns=0)` → `AuxClick(2, btns=0)` |
| 中键 `click` | `MouseDown(1, btns=4)` → `MouseUp(1, btns=0)` → `AuxClick(1, btns=0)`，无 `ContextMenu` |
| 左键 `click` | `MouseDown(0, btns=1)` → `MouseUp(0, btns=0)` → `Click(0, btns=0)` |
| 双击（同一目标，间隔内两次） | 第二次点击追加 `DoubleClick(target 同 Click)`；间隔超 500ms 无 `DoubleClick` |
| ctrl 按住时点击 | `MouseDown` 与 `Click` 均含 `ctrlKey: true` |
| 跨节点（按下 A、抬起祖先 B） | `Click` target 为最近公共祖先；同节点按下抬起时 target 与两者相同 |
| 左键按住 + 右键按下 | 左键 `MouseDown` btns=1，右键 `MouseDown` btns=3（掩码累加，配对互不覆盖） |
| 按住双键拖动 | `MouseLeave` / `MouseEnter` 携带 btns=3 |

引擎载荷形态与附录 Chromium 基准逐项一致：字段编码、事件顺序、按下 / 抬起掩码、多键累加与配对互不干扰。

## 决策记录

| 编号 | 决策 | 日期 |
| --- | --- | --- |
| D1 | `button` / `buttons` 采用 DOM 数字编码（0 / 1 / 2 / 3 / 4 与位掩码）；0-4 之外 DOM 未定义，其他物理键不支持（不产生事件，也不占位）。 | 2026-10-02 |
| D2 | 配对改为按键级（每键独立记录按下目标）+ 最近公共祖先；手势事件在公共祖先上派发，无公共祖先时目标为根。 | 2026-10-02 |
| D3 | `ContextMenu` 对齐 Chromium/Linux：右键按下即派发（target 为按下目标、buttons 含右键位），与抬起和配对无关；不模拟 Windows 的抬起时机。 | 2026-10-02 |
| D4 | `DoubleClick` 由左键点击产生：同目标、间隔 ≤ 500ms、位移 ≤ 4px；阈值属实现定义（参考 Chromium 实测），不实现 `detail`。 | 2026-10-02 |
| D5 | `x` / `y` 在 kit 包装时作为 `clientX` / `clientY` 的别名，不进入引擎载荷。 | 2026-10-02 |
| D6 | 注入键盘时同步 `modifiers_state`，使合成点击能携带修饰键；真实路径不变。 | 2026-10-02 |
| D7 | kit 补齐 `onAuxClick` / `onContextMenu` / `onDoubleClick` 与 `MouseButton` / `MouseButtons` 常量。 | 2026-10-02 |
| D8 | 以 Chromium（Linux）实测为字段取值与事件顺序的对照基准；规范本身留给平台/实现的细节（contextmenu 时机、双击阈值）取实测值，未覆盖场景明确列为不支持。 | 2026-10-02 |
| D9 | `MouseEnter` / `MouseLeave` 的 over/out 语义与冒泡维持现状（修正会破坏 kit 组件），本次仅订正文档，差异记录在案另案处理。 | 2026-10-02 |

## 分阶段实施

### M1：Core 事件与状态（1 天）

- `MouseEvent` / `MouseEventKind` / `PointerButton`（去掉 `Other`）/ `PointerState`；
- 窗口入口忽略 `MouseButton::Other`，按下时携带按键；
- `pointer_press` / `pointer_release`：按下时的 `ContextMenu`、按键级配对、最近公共祖先、`button` / `buttons` / 修饰键填充、DoubleClick。

验收：`cargo build` 通过；事件序列与附录实测记录一致。

### M2：绑定生成与 kit（0.5 天）

- `yarn generate:bindings`；`events/mouse.ts`、`declaration.ts`、常量导出。

验收：kit 类型检查通过。

### M3：调试桥与验证（0.5 天）

- `engine:mouse` 的 down / back / forward；`simulate_key` 同步修饰键；MCP 描述；
- 按"验证"一节实测。

### M4：文档（0.5 天）

- moyu-docs 事件文档更新。

## 影响面与风险

- **性能**：按下 / 抬起是低频路径；`mouse_downs` 定长数组无分配；按下时克隆一次祖先链，与现有 `bubble_target_ids` 的做法一致。
- **行为变化**：见「兼容性」一节（跨节点手势目标、`ContextMenu` 时机、其他物理键的取舍）；当前仓库内无依赖旧行为的代码。
- **测试缺口**：仓库无单元测试，`DoubleClick` 阈值等行为靠 MCP 实测覆盖，不引入新的测试设施。

## 附录：Chromium 实测记录

一次性探测页面挂载 `mousedown` / `mouseup` / `click` / `auxclick` / `contextmenu` / `dblclick` / `mousemove` / `mouseenter` / `mouseleave` 监听，在 Chromium（Linux）中依次执行悬停、左 / 右 / 中键单击、跨元素按下抬起、双击。结论：

| 场景 | 实测结果 |
| --- | --- |
| 悬停移动 | `mousemove` button 0 / buttons 0；`mouseenter` 不冒泡 |
| 左键单击 | `mousedown`(btn 0 / btns 1) → `mouseup`(btn 0 / btns 0) → `click`(btn 0 / btns 0) |
| 右键单击 | `mousedown`(btn 2 / btns 2) → `contextmenu`(btn 2 / btns 2) → `mouseup`(btn 2 / btns 0) → `auxclick`(btn 2 / btns 0)；无 `click` |
| 中键单击 | `mousedown`(btn 1 / btns 4) → `mouseup`(btn 1 / btns 0) → `auxclick`(btn 1 / btns 0)；无 `click`、无 `contextmenu` |
| 跨元素左键 | down 于子节点、up 于祖先空白 → `click` target 为祖先（最近公共祖先）；无重复 `click` |
| 跨元素右键 | down 于子节点（`contextmenu` 立即在子节点触发）→ up 于祖先 → `auxclick` target 为祖先 |
| 双击 | `mousedown` → `mouseup` → `click` ×2 → `dblclick`；第二次 `click` 照常派发；`dblclick` 的 `detail` 为 2、buttons 0 |

（探测页面为一次性验证材料，不纳入版本控制；复现方式：浏览器中挂上述监听并按相同顺序执行动作。）
