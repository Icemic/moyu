# RFC：触摸的鼠标兼容事件合成

- **状态**：已接受
- **日期**：2026-10-03
- **作者**：末语项目组
- **适用范围**：`moyu_core`（`crates/core`）、`@momoyu-ink/kit`、视觉小说框架（`moyu-framework`、`moyu-framework-fishflow`）
- **相关实现**：`crates/core/src/core/pointer_events.rs`、`crates/core/src/state.rs`、`packages/kit/src/components/button.tsx`、`packages/kit/src/events/globals.ts`、`packages/kit/src/state.ts`
- **相关 RFC**：[运行时输入注入](./2026-10-02-input-injection.md)、[指针按键](./2026-10-02-pointer-buttons.md)

## 摘要

引擎的输入模型里鼠标与触摸是两条互不相干的路径：`MouseDown` / `MouseUp` / `Click` 只由鼠标指针（`MOUSE_IDENTIFIER`）产生，触摸只产生 `TouchStart` / `TouchMove` / `TouchEnd` / `TouchCancel`。浏览器会给触摸补发一套「鼠标兼容事件」（compatibility mouse events），引擎没有对应机制。

后果是：所有挂在 `onClick` 上的交互在触摸设备上完全不响应。视觉小说框架的遮罩关闭、历史记录条目、标题页跳过入场、错误页按钮都属于这一类；kit 为了绕开这一点，在 `Button` 里给触摸单独接了一遍 `onPress`，又带来新的一致性问题（同一手势在触摸与鼠标两条路径上重复触发）。

本文在引擎输入层补上这套兼容事件：触摸是 tap 时，把鼠标指针移到抬手位置并复用既有的鼠标手势管线（移动 / 按下 / 抬起），产生与鼠标点击一致的事件序列，包括 hover 状态、光标、双击识别与 `pointer_event`。kit 与框架中为绕开这一点而存在的触摸特化代码随之删除。

## 背景

### 现状

| 位置 | 现状 | 触摸下的结果 |
| --- | --- | --- |
| 引擎 `pointer_events.rs` | 鼠标手势只由 `MOUSE_IDENTIFIER` 的 `pointer_press` / `pointer_release` 产生 | 触摸不产生鼠标事件 |
| kit `Button` | 自行处理 `onTouchStart` / `onTouchEnd`，`onTouchEnd` 直接调 `onPress` | `onPress` 可用，但触摸处理器不拦截冒泡 |
| kit 其它控件（Checkbox / Radio / Select / Slider / Input） | 基于 `Button` 或自行处理触摸 | 同上 |
| 框架组件树 | 遮罩、列表条目、文本预览等只挂 `onClick` | 完全不响应 |
| 框架 `stage.tsx` | 全局 `touchend` 与 `click` 都调 `handleClick` | 点 kit 按钮时，`onPress` 与全局 `touchend` 同时命中，产生重复推进、按钮被抵消、面板开关反转（详见下节） |

重复触发的具体表现：点文本框关闭按钮会立刻被逻辑重新打开；点 AUTO 会先 `startAuto()` 再被 `stopAuto()` 抵消；点选项按钮会连续 `nextLine()` 两次。原因是 kit 的 `Button` 在桌面端靠内建的 `onClick` 处理器调 `stopPropagation()` 拦住事件冒泡到 Stage 的全局 `click` 监听，而触摸事件没有对应的拦截。

### 为什么不在 kit 或框架层合成

kit 里曾有一版 JS 层合成（`2a3d1eb` 加入，现已注释停用，只留下 `STATE.touchMoved` 的状态跟踪）。这条路径解决不了三个问题：

1. **hover 与光标状态在引擎里**。`handle_pointer_hover` 对非鼠标指针只更新命中目标与光标，`MouseMove` / `MouseEnter` / `MouseLeave` 只在 `identifier == MOUSE_IDENTIFIER` 时派发。JS 层伪造的 `onMouseEnter` 不会改变引擎的 hover 状态，后续鼠标事件与富文本链接的交互会对不上。
2. **click 的目标语义在引擎里**。`pointer_release` 里已经实现了「按下与抬起的最近公共祖先」作为 click 目标、双击识别与 `buttons` 掩码，JS 层重写一遍必然与鼠标手势漂移。
3. **合成必然与 kit 的触摸路径重复**。只要 `Button.onTouchEnd` 还调 `onPress`，引擎补发的鼠标手势就会再触发一次 `onPress`。

框架侧逐个补 `onTouchStart` 的问题是同一件事在 N 处各写一遍，且 hover 显隐类 UI 无法用这种方式补齐。

## 目标

- 触摸 tap 产生与鼠标点击一致的事件序列：`MouseMove`、`MouseEnter` / `MouseLeave`、`MouseDown`、`MouseUp`、`Click`，以及对应的 `pointer_event`（含富文本链接所需的 `Click`）；
- 拖动、多指、取消不产生兼容事件；
- 鼠标输入的行为与现在完全一致；
- `debug_touch` 注入一次 tap 的效果与真人触摸一致，结果里能看到兼容事件名。

## 非目标

- **长按 → `ContextMenu`**：移动端的长按菜单不在本文范围，需要时另开；
- **手势识别**：pinch / swipe / 长按的通用识别不做，本文只做 tap 与鼠标兼容事件之间的映射；
- **JS 侧 `preventDefault()` 抑制合成**：引擎到 JS 的事件派发是单向调用，拿不到返回值。浏览器里「JS 阻止了 touch 事件就不再补发鼠标事件」这条规则本次不实现，当前 kit 与框架也没有依赖它的地方；
- **触摸期间的连续鼠标跟随**：只在 tap 结束时合成一次，touchmove 不产生 `MouseMove`（浏览器也只补发一次）。

## 设计

### tap 判定

`PointerState` 增加一个只对触摸有效的字段：

```rust
/// Tracked while a touch is down, so that `End` can decide whether the touch is a tap
/// that the mouse compatibility gesture is synthesized for.
pub(crate) struct TapGesture {
    /// Where the touch started, in stage logical coordinates.
    pub start_x: f32,
    pub start_y: f32,
    /// Set once the touch moves past the tap slop, or another touch becomes active.
    pub cancelled: bool,
}
```

成为 tap 的条件（全部满足才合成）：

| 条件 | 说明 |
| --- | --- |
| 位移未超过 slop | 起点到任意一次 `TouchMove` 的距离不超过 `TAP_SLOP`（常量放在 `pointer_events.rs`，与 `DOUBLE_CLICK_DISTANCE` 并列，初值 12 逻辑像素） |
| 没有其它活动触点 | 手势期间同时存在多个触点即全部取消 |
| 不是 `TouchCancel` | 系统取消的手势不合成 |

slop 判定放在 `TouchPhase::Move` 里并置 `cancelled`：手指移出再移回时，只在 `End` 比较起点与终点会误判为 tap。

### 合成时机与顺序

`pointer_touch` 的 `TouchPhase::End` 分支，在派出 `TouchEnd` 与清空 `touch_down_id` 之后，若该触摸是 tap，则复用 `simulate_pointer` 的 `Click` 动作顺序：

```rust
// A tap produces the mouse compatibility gesture browsers would: move the mouse
// pointer to where the finger lifted, then run the ordinary press / release pair so
// hover state, cursor, click target and double click detection all follow the mouse path.
self.pointer_to(MOUSE_IDENTIFIER, location.client_x as f32, location.client_y as f32);
self.handle_pointer_hover(MOUSE_IDENTIFIER, true, record);
self.pointer_press(MOUSE_IDENTIFIER, PointerButton::Left, record);
self.pointer_release(MOUSE_IDENTIFIER, PointerButton::Left, record);
```

顺序与浏览器一致：`TouchEnd` 先派发给 JS，兼容鼠标事件在其后。合成前先把鼠标指针移到抬手位置，因此按下与抬起发生在同一点，click 目标就是抬手处的命中节点，与浏览器把整套兼容鼠标事件都发在 `touchend` 位置一致；kit 的 `onClick` + `stopPropagation()` 拦截对合成事件同样生效，框架 Stage 的全局 `click` 监听不会再被重复命中。

合成落在鼠标指针上（`MOUSE_IDENTIFIER`），与浏览器一致：一次 tap 之后引擎的鼠标指针停在抬手位置，后续鼠标事件的起点也随之改变。

### 多指与取消

第二个触点按下时，连同已在按的触点在内部置 `cancelled`，全部不合成。这样两指以上的手势（哪怕是双指轻触）都不会产生 stray click。

实现约束：`pointer_touch` 全程持有该指针的 `DashMap` 分片锁（`get_pointer_state_mut!`），在持锁状态下遍历 `pointer_map` 会阻塞在同一个分片上。因此多指取消在获取指针状态之前完成，作为一个独立的辅助函数返回「是否已有其它活动触点」，供 `Start` 分支写入 `cancelled`。

### 触摸事件本身不删除

`TouchStart` / `TouchMove` / `TouchEnd` / `TouchCancel` 与 `pointer_event`（`Down` / `Up` / `Over` / `Leave`）照旧派发，兼容事件与它们并存：

- 拖动类交互（kit 的 `ScrollView` 滚动、`Slider` 拖动、backlog 滚动条拖动）继续用触摸事件，与浏览器一致；
- 引擎的 `Editable` 聚焦在 `TouchStart` 时已经完成，合成的 `MouseDown` 再次调用 `handle_pointer_down` 对同一目标是幂等的（`EditableManager::focus` 对已聚焦目标提前返回）；
- 一个 tap 里富文本链接会收到触摸路径的 `Down` / `Up` 与鼠标路径的 `Down` / `Up` / `Click`，与浏览器同时派发 `pointerdown` 与 `mousedown` 的情形对应，消费方按 `kind` 过滤。

### kit 的配套删减

| 文件 | 改动 | 原因 |
| --- | --- | --- |
| `components/button.tsx` | `onPress` 改由 `Click` 触发，`onMouseUp` / `onTouchEnd` 只负责按压视觉 | 按钮的"按下并在同一目标上抬起"就是引擎的 click 手势，无需在组件里重建 |
| `components/button.tsx` | 保留 `onTouchStart` 置 `pressed` 与全局 `touchend` / `touchcancel` 复位 | 手指按下期间需要按压视觉，而合成的 `MouseDown` 只在抬手时到达 |
| `components/slider.tsx` | 按下轨道即刻跳值并从该位置开始拖动，删除只在释放时用到的 `moved` / `startedOnThumb` | 拖动与点按合并成一条路径，不再需要区分二者 |
| `components/slider.tsx` | 拖动只跟随发起它的指针设备（用 `pointerIdentifier` 比对） | 引擎每帧重发 `MouseMove`，而触摸与鼠标指针是两个独立的指针，不区分会互相覆盖 |
| `events/globals.ts` | 删除注释掉的 JS 合成代码与 `STATE.touchMoved` 跟踪 | 该机制被本文的引擎实现取代，留着会误导 |
| `state.ts` | 删除 `touchMoved` 字段 | 同上 |
| `ScrollView` / `Input` | 不动 | 它们的滚动与聚焦已使用每次移动都会刷新的字段 |

`Checkbox` / `Radio` / `Select` 基于 `Button`，无需单独改动。

### 框架的配套删减

| 仓库 | 文件 | 改动 |
| --- | --- | --- |
| `moyu-framework` | `src/pages/stage.tsx` | 删除全局 `touchend → handleClick`，只保留 `click`；否则一次 tap 会推进两行 |
| `moyu-framework` | `src/actors/textbox.tsx` | 文本框功能按钮目前只在鼠标悬停时显示，触摸设备没有 hover，按触摸输入常驻显示（见下） |
| `moyu-framework-fishflow` | `src/pages/stage.tsx` | 同上删除全局 `touchend` |
| `moyu-framework-fishflow` | `src/actors/textbox.tsx` | 同上；该仓库有 `controls.hover.showOnHover` 配置，触摸检测与它叠加 |

其余页面（backlog / menu / saveload / settings / title / error）无需改动，它们的 `onClick` 由合成事件覆盖。

按钮常驻显示的做法：监听全局 `touchstart` 进入触摸模式，`mousemove` 退出触摸模式；触摸模式下忽略 `showOnHover` 判定，按钮组始终可见。全局 `touchstart` / `mousemove` 的区分在两种平台都成立（web 端 winit 按 `pointerType` 分流触摸与鼠标指针，触摸不会产生 `mousemove`）。

## 决策记录

| 编号 | 决策 | 日期 |
| --- | --- | --- |
| D1 | 兼容事件在引擎输入层合成，不在 kit 或框架层伪造；复用 `pointer_press` / `pointer_release`，使 hover、光标、click 目标与双击识别的行为与鼠标一致。 | 2026-10-03 |
| D2 | 合成时机为 `TouchEnd` 之后，目标位置取抬手位置；合成落在鼠标指针上。 | 2026-10-03 |
| D3 | tap 判定基于起点到任意一次 move 的位移，超过 `TAP_SLOP` 即取消；多指与 `TouchCancel` 一律取消。 | 2026-10-03 |
| D4 | 触摸事件与 `pointer_event` 照旧派发，兼容事件是叠加；拖动类交互继续走触摸路径。 | 2026-10-03 |
| D5 | kit 删除 `Button.onTouchEnd → onPress` 与 `STATE.touchMoved` 残留，保留触摸按压视觉。 | 2026-10-03 |
| D6 | 不实现长按 → `ContextMenu`、手势识别与 JS `preventDefault()` 抑制合成。 | 2026-10-03 |
| D7 | `Button.onPress` 由引擎的 `Click` 触发：引擎已经算好了手势目标，组件侧用 `pressed` 状态判断会因为同轮次下发而读到未更新的值，用 ref 补那么一份状态等于把这个概念重建一遍。按压视觉仍由 `onMouseDown` / `onTouchStart` 驱动。 | 2026-10-03 |
| D8 | `Slider` 的拖动增量改用 `clientX`（舞台坐标）：`offsetX` 相对的是命中目标（同 DOM 的 `event.target`），指针移到别的节点上时参考系会变，增量会跳。 | 2026-10-03 |
| D9 | `Slider` 在按下轨道时即刻跳值并从该位置开始拖动，不再区分"点按"与"拖动"；拖动只跟随发起它的指针设备，因为引擎每帧重发 `MouseMove`，而触摸与鼠标是两个独立指针。 | 2026-10-03 |

## 分阶段实施

### M1：引擎合成（本仓库）

- `crates/core/src/state.rs`：`PointerState` 增加 tap 状态；
- `crates/core/src/core/pointer_events.rs`：`TAP_SLOP`、slop 判定、多指取消、`End` 分支的合成；
- 核对 `simulate_touch` 与窗口事件两条入口共用该逻辑。

验收：`cargo build` 通过；`debug_touch` 的 tap 结果里出现 `MouseMove`、`MouseDown`、`MouseUp`、`Click`；`debug_mouse` 行为不变。

实施记录（2026-10-03）：已完成。`TAP_SLOP` 取舞台逻辑像素，与 `DOUBLE_CLICK_DISTANCE` 同一坐标系（`handle_pointer_move` 已把窗口物理位置换算为舞台逻辑坐标）。验证用 `debug_start` 的 attach 模式接本地构建的 `target/debug/moyu`，加载 `moyu-framework` 的 dev server：

| 用例 | 实际 dispatched |
| --- | --- |
| 单击 tap | `TouchStart`、`TouchEnd`、`MouseMove`、`MouseLeave`、`MouseEnter`、`MouseDown`、`MouseUp`、`Click` |
| 位移超过 slop | `TouchStart`、`TouchMove`、`TouchEnd` |
| 位移在 slop 内 | 与单击 tap 相同 |
| 两指轻触（两次 `start`，各自 `end`） | 只有 `TouchStart` / `TouchEnd`，无鼠标事件 |
| `cancel` | `TouchStart`、`TouchCancel` |

探针核对事件目标：tap 标题页「开始游戏」按钮时 `Click` 的 target 是按钮的 sprite 节点，冒泡链为按钮容器 → 按钮组 → `content` → `title`，页面随即进入 stage，说明框架的 `onPress` 已在触摸下生效。`debug_mouse` 的 `click` 仍是 `MouseMove`、`MouseDown`、`MouseUp`、`Click`，与改动前相同。

### M2：kit 与框架

- kit：按上表删减触摸特化代码，`onPress` 改由 `Click` 触发；
- `moyu-framework` / `moyu-framework-fishflow`：删除全局 `touchend → handleClick`，补按钮常驻显示；
- 文档：框架 `AGENTS.md` 补一句 tap 会一并产生兼容事件，kit `AGENTS.md` 补一句控件不要从 `onTouchEnd` 触发点击动作。

验收：Android 真机上点历史记录条目、设置页滑块与复选框、文本框按钮、选项按钮各一次且不重复触发；桌面鼠标行为不变。

实施记录（2026-10-03）：已完成，桌面路径用 `debug_mouse` / `debug_touch` 逐项验证，Android 真机留作 M2 之后的手工检查。

kit 侧除按上表删减外，`Button.onPress` 改为由引擎的 `Click` 触发。先前的实现用 `onMouseUp` + 一个按压状态判断，遇到的问题是：引擎把合成的 `MouseDown` 与 `MouseUp` 放在同一轮次下发，React 尚未重渲染，`MouseUp` 读到的 `pressed` 仍是旧值，`onPress` 不触发；当时用一个 ref 绕过去了，但那份 ref 实际上是在组件里重建"按下与抬起在同一目标"这个引擎已经算好的概念。改用 `Click` 后 ref 消失，且更接近浏览器的行为：指针中途移出再回到按钮上释放，依然算一次点击。

`Slider` 因此不再用 `onPress` 完成点按跳值，改为按下轨道即刻跳值并从该位置开始拖动（原生滑块的行为），跳值与拖动合成一条路径，删掉了 `moved` / `startedOnThumb` 两个只在释放时用到的字段。

另一个实测发现：**引擎每帧重发 `MouseMove`，会把触摸拖动带飞**。触摸按住轨道开始拖动后，若鼠标指针停在轨道上，每帧的 `MouseMove` 也会进 `moveDragging`，用鼠标的位置重算数值，把拖动值覆盖掉（实测：触摸跳到 0.2201 后立即被改回 0.5013，对应当鼠标停在 x=706 与按下点 x=600 的差值）。修正方式是让拖动只跟随发起它的指针设备：记录发起拖动的是鼠标还是哪个触摸标识，不匹配的事件一律忽略。这是既有实现的缺陷（之前的 `'identifier' in event` 判断只在鼠标拖动时拦住触摸事件，反方向拦不住），只是因为之前测试时鼠标与手指同位置而未暴露。

文本框按钮组的触摸常驻显示比预想复杂：引擎在桌面端每帧都会重新派发一次 `mousemove`（位置不变），因此「收到 mousemove 就退出触摸模式」会在下一帧立刻失效。最终按位置是否变化判断：只有鼠标移动到新位置才算用户在用鼠标。

| 用例 | 实际结果 |
| --- | --- |
| tap 文本框 / 选项按钮 / 文本框功能按钮 / 标题页按钮 | 均只触发一次；`onPress` 生效 |
| tap 历史记录条目 | 弹出跳转确认框，跳转后 gameState 恢复 |
| tap 确认框按钮 | 对话框关闭，backlog 一并收起 |
| tap 文本框 LOG 按钮 | 打开 backlog；剧情记录数不变（未误推进） |
| 触摸拖动 backlog 内容 | 内容位移从 -136 变为 0；未触发条目跳转（位移超过 slop 不合成 click） |
| tap 设置页滑块轨道 | 音量变为 `(offsetX - 手柄半宽) / 轨道长度` 的精确值（例如轨道上 x=800 处 tap 得到 0.7506631 = (295 − 12) / 377） |
| 触摸拖动设置页滑块 | 从 0 拖动 100 像素后音量变为 0.2652520 = 100 / 377；鼠标停在轨道上时，按下处 x=600 得到手柄 x=83 = (95 − 12)，继续拖到 x=750 后手柄 x=233，与手指位移一致 |
| 鼠标拖动设置页滑块 | 按下 x=700 跳到手柄 x=183，拖到 x=650 后手柄 x=133，与鼠标位移一致 |
| 触摸 tap 文本框后再触摸 | 按钮组保持可见 |
| 触摸后移动鼠标 | 退出触摸模式，恢复 hover 显隐 |

**滑块拖动**：实施过程中曾观察到「触摸拖动不生效」，后续用事件探针复核，该观察是读数失误（读到的节点不跟随滑块值变化），引擎与控件都没有缺陷。

**拖动的坐标系**：事件探针实测确认，引擎在触摸的 `TouchMove` / `TouchEnd` 上会按固定目标重新计算 `offset`（手指从 x=706 移到 606 时 `offsetX` 由 201 变为 101）。需要改掉 `offsetX` 的原因在于它相对的是**命中目标**（与 DOM 中相对 `event.target` 一致），而这个目标在指针移到别的节点上时会变：鼠标按住轨道后向左移出 266 像素，`offsetX` 从 201 变为 115（目标由轨道变为上层节点），用它算出的增量是 −86，与实际位移不符。拖动增量因此改用参考系稳定的 `clientX`（舞台坐标）；"按下点是否在手柄上"仍用 `offsetX`，因为那本来就是相对轨道的定位问题。

**生效范围**：`moyu-framework` 通过相对路径依赖本地 kit，改动立即生效；`moyu-framework-fishflow` 依赖已发布的 `@momoyu-ink/kit`，其框架侧改动要等下一次 kit 发布后才会生效。

## 影响面与风险

- **性能**：tap 结束时多做一次命中测试与四个事件派发，量级与一次鼠标点击相同。
- **粘性 hover**：tap 之后鼠标指针停在抬手位置，被点的节点保持 hover 视觉，直到下一个 tap 或鼠标移开。与浏览器在触摸设备上的表现一致。
- **双击**：同一位置快速两次 tap 会触发 `DoubleClick`，与浏览器一致；当前 kit 与框架没有消费方。
- **滑块提交两次**：tap 滑块轨道时，`touchend` 先提交一次拖动（值为原值），合成 click 再跳到目标位置并提交一次。最终值正确，框架未使用 `onValueCommit`。
- **Windows 触摸**：winit 的 Windows 后端按 `WM_POINTER*` / `WM_TOUCH` 上报触摸。若系统同时投递「提升为鼠标」的 `WM_LBUTTONDOWN` 一类消息，一次触摸会同时产生系统合成的鼠标手势与本文合成的鼠标手势。需要在 Windows 触摸设备上实测，确认存在重复时在窗口事件入口按 `MOUSEEVENTF_FROMTOUCH`（`dwExtraInfo` 高位 `0xFF515700`）过滤。该情形在改动前就已经是「触摸与鼠标两条路径各触发一次」，本文不会让它变差，但需要在 M2 之前确认。
- **平台一致性**：native（Android / iOS）与 web 都走 `WindowEvent::Touch`，行为一致；web 端 winit 只监听 `pointer*` 与 `touchstart`，不监听 DOM 的兼容鼠标事件，不会与本文的合成重复。

## 验证

| 场景 | 期望 |
| --- | --- |
| `debug_touch` 单点 tap | `dispatched` 含 `TouchStart`、`TouchEnd`、`MouseMove`、`MouseDown`、`MouseUp`、`Click` |
| `debug_touch` 移动超过 slop 后抬手 | 只有 `TouchStart` / `TouchMove` / `TouchEnd`，无鼠标事件 |
| `debug_touch` 两指轻触 | 无鼠标事件 |
| `debug_touch` `cancel` | 无鼠标事件 |
| Android 真机点历史记录条目 | 弹出跳转确认，不误触发滚动 |
| Android 真机点文本框关闭按钮 | 隐藏后不被重新打开 |
| Android 真机点选项按钮 | 只推进一次 |
| 桌面鼠标全部交互 | 与改动前一致 |
