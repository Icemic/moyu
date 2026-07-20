# @momoyu-ink/kit — Agent 指南

本文档面向 AI coding agent 和开发者，描述末语引擎 JavaScript SDK（`@momoyu-ink/kit`）的架构与开发规范。

> 引擎核心与通用规范见仓库根 [AGENTS.md](../../AGENTS.md)。本文件只覆盖 kit 包内约定。

---

## 概述

`@momoyu-ink/kit` 是末语引擎的官方 React SDK，提供：

- 自定义 **React Reconciler**，将 React 组件树映射到引擎节点树
- 引擎通信层（`moyu.executeNodeCommand` / `executePluginCommand` 等）
- 事件系统（冒泡事件 + 全局事件）
- 基于 `react-spring` 的动画
- Stage / Scenario hooks（剧情运行时集成）
- Stack 风格的导航组件（pages + overlays）
- 基于 `valtio` 的 sandbox（JS 沙盒变量访问）
- 由 `ts-rs` 从 Rust 生成的类型绑定

**关键环境：**

- **浏览器（Web）**：完整 Web API
- **QuickJS（Native）**：受限运行时，kit 内置必要 polyfill

---

## 目录结构

```
packages/kit/src/
├── lib.ts                   # 主入口，重新导出公共 API
├── moyu.ts                  # 引擎通信底层（createInstance / updateProps / executeXxxCommand ...）
├── node.ts                  # Node 节点封装类
├── react.ts                 # React Reconciler 实现
├── declaration.ts           # JSX 命名空间与 IntrinsicElements
├── commands.ts              # 重新导出 bindings 中的 Command 类型
├── events.ts                # 事件系统总入口
├── events/                  # 事件实现
│   ├── base.ts              # BubbleEvent 基础类型
│   ├── mouse.ts / touch.ts / keyboard.ts / wheel.ts
│   ├── globals.ts           # 全局事件分发
│   └── listener.ts          # addEventListener 实现
├── hooks.ts                 # hooks 总入口
├── hooks/
│   ├── useStage.ts          # createStage / StageContextProvider / GameControl
│   ├── useScenario.ts       # useScenario / nextLine / setWaiting
│   ├── useFadeInOut.ts      # 淡入淡出辅助
│   └── useSoundEffect.ts    # 音效便捷 hook
├── components/
│   └── navigation.tsx       # createStackNavigator / Navigation / useNavigation
├── spring/                  # react-spring 集成
│   ├── index.ts
│   ├── animated.ts          # animated.<element> 原语
│   └── primitives.ts
├── state.ts                 # 内部状态
├── sandbox.ts               # ARCHIVE/GLOBAL 变量 Proxy（通过 scenario 插件访问剧本变量）
├── runtime-globals.ts       # Console / 全局 API 类型声明
├── utils.ts                 # 工具函数
├── zod-patch.ts             # Zod 兼容补丁
├── jsx-runtime.ts           # React 19 自动 JSX runtime
├── jsx-dev-runtime.ts       # dev 模式 JSX runtime
└── bindings/                # 由 `yarn generate:bindings` 从 Rust 生成，勿手工改
    ├── NodeProps.ts / SpriteProps.ts / TextProps.ts / VideoProps.ts / ...
    ├── AudioCommand.ts / ScenarioCommand.ts / SystemCommand.ts / ...
    └── ...
```

**重要规则：**

- `bindings/` 由 `yarn generate:bindings`（运行 `cargo test export_bindings --workspace`）自动生成，**禁止手工修改**。修改对应的 Rust 结构体后必须重新生成。
- `lib.ts` 会同时重新导出 `./spring` 与 `@react-spring/core`，并通过 package exports 暴露 ESM / CJS 双入口；改动打包或导出配置时要同时检查 `tsup.config.ts` 与 `package.json`。

---

## 核心模块

### 1. React Reconciler（`react.ts`）

kit 实现了完整的 React Reconciler，将 React 组件映射到引擎节点：

```typescript
const hostConfig: HostConfig<...> = {
  createInstance(type, props) {
    return Node.create(props.label ?? '', type, props);
  },
  appendChild(parent, child) { parent.addChild(child); },
  // ...
};
```

**重要约束：**

- **不支持原生 HTML 元素**（`div` / `span` 等会崩溃）
- 只支持 Moyu 特有元素（见下方 "内置组件"）
- 使用 `createRoot()` 而非 `react-dom` 的 `createRoot`

### 2. 引擎通信层（`moyu.ts`）

通过全局 `moyu` 对象与 Rust 引擎通信：

```typescript
// 底层命令
moyu.pushCommand(name, args, callback);        // 通用命令
moyu.executeNodeCommand(nodeId, payload);       // 节点命令
moyu.executePluginCommand(pluginName, payload); // 插件命令
```

**封装 API（kit 导出）：**

- 节点生命周期：`createInstance` / `destroyInstance`
- 节点树：`addChild` / `insertChild` / `insertChildBefore` / `removeChildAt` / `removeChild`
- 属性：`updateProps`
- 命令：`executeNodeCommand` / `executePluginCommand`

### 3. 事件系统

引擎从 Rust 层通过全局 `__moyu_receive_event` 回调把事件送到 JS 层，kit 负责分发。

**事件类别：**

- **冒泡事件**：`MouseEvent` / `TouchEvent` / `KeyboardEvent` / `WheelEvent`，从目标节点向上冒泡
- **节点事件**：`NodeEvent`（创建、销毁）
- **全局事件**：通过 `addEventListener(type, handler)` 监听

**支持的全局事件类型**（`addEventListener(type, handler)`，`type` 为纯字符串，分发逻辑见 `events/globals.ts`）：

- **冒泡事件的全局落点**：引擎 top-level name 为 `mouseevent` / `touchevent` / `keyboardevent` / `wheelevent`；分发时按 `body.kind`（PascalCase）调节点 `on<Kind>`，冒泡完成后按 `kind.toLowerCase()` 触发全局监听器。因此全局监听用小写名，如 `click` / `mousedown` / `mousemove` / `keydown` / `wheel` / `touchstart` 等。
- **自定义事件**（`customevent`）：按 `body.name` 分发；`targetId === 0` 时触发 `globalEventListeners[name.toLowerCase()]`，否则派发到节点的 `on<Name>`。
- **节点生命周期**（`nodeevent`）：内部用于清理 `nodeMap`，**不暴露给用户监听**。
- **RAF 驱动**（`animationframecallbackevent`）：消耗 `globalRequestAnimationFrameListeners`，支撑 `requestAnimationFrame` polyfill，**不暴露给用户监听**。
- **其它由引擎下发的 name**：通过 `default` 分支直接查 `globalEventListeners[name]`（如 `ready`、`resize`、`fullscreen`、`beforeunload`、`gamepad` 等，具体取决于引擎发射集合）。

具体事件字段的定义见 `bindings/`（`KeyboardEventKind` / `MouseEventKind` / `TouchEventKind` / `GameEvent` / `ResizeEvent` 等）。

```typescript
import { addEventListener } from '@momoyu-ink/kit';

const unregister = addEventListener('keydown', (event) => {
  // handle keyboard event
});
```

### 4. 动画系统

基于 `react-spring` 的声明式动画：

```typescript
import { animated, useSpring } from '@momoyu-ink/kit';

function Fade() {
  const styles = useSpring({ opacity: 1, from: { opacity: 0 } });
  return <animated.sprite style={styles} src="image.png" />;
}
```

**关键原则：由 valtio / 外部 state 驱动的动画使用 reactive 模式。**

```typescript
// ✅ 推荐：reactive 模式，state 变化自动重新动画
const springs = useSpring({
  x: character.x,
  y: character.y,
  config: { duration: 500 },
});

// ⚠️ factory 模式（`useSpring(() => ({...}))`）需要手动 `api.start()`
// 触发，与 valtio 频繁重渲染配合时容易出现时序问题。仅在按钮点击、
// 动画链等手动触发的场景使用。
```

`useTransition` 只跟踪 item 的 enter / leave / update by key，不会传播已有 item 的属性变化。如需响应 item 属性变化，在子组件内部用 `useSnapshot()` 读最新 state。

### 5. Stage & Scenario（`hooks/useStage.ts` + `hooks/useScenario.ts`）

Stage 是剧情驱动 UI 的核心抽象：命令调度、流程控制、skip / auto 状态，以及自动模式下的 barrier / ticket 协调。

**常用 API：**

- `createStage()` — 创建 stage 实例，负责注册命令处理器、文本处理器和模式控制逻辑
- `StageContextProvider` / `useStageContext()` — 在 actor 树中共享同一个 stage
- `useScenario(stories, startName?, entryName?, goNextOnLoad = false)` — 加载剧情并接管生命周期
- `nextLine()` / `setWaiting(time, skippable)` — 直接调用 scenario 插件推进或进入等待
- `useSkipCallback(fn)` / `useInterruptCallback(fn)` — 注册 skip 收尾逻辑和用户点击打断逻辑
- `useSkipBlocker(fn)` / `useAutoBlocker(fn)` — 返回 `true` 时阻止 skip / auto 启动或继续
- `useBeforeHandleCommandCallback(fn)` — 在每条 `scenariocommandline` 分发前执行
- `useAutoTicket()` — 为当前 auto barrier 发放完成 ticket
- `useIsSkipping()` / `useIsAutoing()` — 读取 `skipState` / `autoState` 的响应式快照
- `setDefaultAutoTailMs(ms)` — 设置 auto 模式默认尾延迟，用于 fallback 或新 ticket 的 `tailMs`

**GameControl**（传给命令 handler）：

| 方法 | 作用 |
|------|------|
| `control.hold()` | 暂停直到用户操作；在 auto 模式下会打开 `hold` barrier |
| `control.setWaiting(ms, skippable)` | 定时等待；在 auto 模式下会打开 `wait` barrier |
| `control.nextLine()` | 立即推进 |
| `control.unskippable()` | 将本次 dispatch 标记为不可跳过 |
| `control.record(meta)` | 记录当前运行时快照到 backlog，并返回 record id |

**实现要点：**

- Stage 当前只封装 `scenariocommandline` 和 `scenariotext` 两类事件；`ResolvedSystemCallLine` 虽然从 `events.ts` 导出，但若要处理 system call，需要自己显式监听对应事件。
- auto 模式下，`hold()` / `setWaiting()` 不会直接把控制权交给普通等待逻辑，而是打开一个 barrier。只有当 barrier 的 ticket 全部 `done()` / `cancel()`，并且各自 `tailMs` 都结算完成后，Stage 才会恢复 `nextLine()`。
- `useAutoTicket()` 允许 actor 参与 auto barrier 协调。若 ticket 在 barrier 打开前的短暂采集窗口内创建，Stage 会先把它放入 pending 集合，再在下一个 barrier 打开时收编，解决语音等副作用早于文本 barrier 注册的问题。
- skip / auto blocker 用于表达“当前流程禁止模式继续”的业务条件，适合选项菜单、模态交互等必须等待用户操作的场景。

`useScenario` 内部维护一个带 session key 的 refcount session。key 由 `stories`、`startName`、`entryName`、`goNextOnLoad` 共同决定；相同 key 的瞬时 remount 会复用同一个 live session。cleanup 时只会减少 `refCount`，真正的 `terminateStory` 会被排入串行生命周期队列，并推迟到下一个宏任务，这样 Fast Refresh 之类的短暂卸载不会重置剧情；只有 key 变化或最终卸载时，旧 session 才会被终止。

### 6. 导航（`components/navigation.tsx`）

Stack 风格导航，区分 **pages**（全屏）和 **overlays**（模态）：

```typescript
import { createStackNavigator, createStaticNavigation, RegisterNavigator } from '@momoyu-ink/kit';

const navigator = createStackNavigator({
  pages: { title: TitlePage, stage: StagePage },
  overlays: { menu: MenuOverlay, settings: SettingsOverlay },
  initialPage: 'title',
});

export const Navigation = createStaticNavigation(navigator);

declare module '@momoyu-ink/kit' {
  interface RootNavigatorList extends RegisterNavigator<typeof navigator> {}
}
```

- `useNavigation()` / `getNavigator()` — 组件内/外部导航
- `useNavigationParams<T>()` — 读取当前页面/overlay 参数
- 通过 module augmentation 扩展公开模块 `@momoyu-ink/kit` 中的 `RootNavigatorList`，不要去扩展内部相对路径模块

### 7. Sandbox（`sandbox.ts`）

Sandbox 为剧本表达式提供受控求值环境：

- `ARCHIVE` 代理到 scenario 插件的普通变量（`getVariable` / `setVariable`）
- `GLOBAL` 代理到 scenario 插件的永久变量（`getPermanentVariable` / `setPermanentVariable`）
- `__moyu_eval_sandbox` 会在代理作用域里执行表达式，保留 JS 内建全局的透传
- 在 sandbox 内部，`window` 和 `globalThis` 都会指向 sandbox proxy 自身，而不是浏览器真实全局对象；编写表达式或运行时辅助逻辑时不能假设它们等同于宿主环境

### 8. Bindings（`bindings/`）

由 Rust 用 `ts-rs` 自动生成的 TypeScript 类型。涵盖：

- 节点 Props（`NodeProps` / `SpriteProps` / `TextProps` / `VideoProps` / `ClipProps` / `FilterProps` / `BackdropProps` / `AnimationProps`）
- 命令（`AudioCommand` / `TextCommand` / `ScenarioCommand` / `SystemCommand` / `GamepadCommand` / `VideoCommand`）
- 事件（`KeyboardEvent` / `MouseEvent` / `TouchEvent` / `WheelEvent` / `GameEvent` / `GamepadEvent` / `ScenarioEvent` / `TextEvent` / ...）
- 其它类型（`WindowState` / `AudioSettings` / `MoyuCursor` / `SpriteMode` / `NineSliceMode` / ...）

---

## 内置组件

### JSX IntrinsicElements

| 组件          | 用途     | 关键 Props                                   |
| ------------- | -------- | -------------------------------------------- |
| `<container>` | 容器     | 基础变换属性                                 |
| `<vbox>`      | 纵向布局 | `width`, `height`, `gap`, `padding`, `justifyContent`, `alignItems` |
| `<hbox>`      | 横向布局 | `width`, `height`, `gap`, `padding`, `justifyContent`, `alignItems` |
| `<sprite>`    | 图片精灵 | `src`, `area`, `mode`, `bounds`              |
| `<text>`      | 文本     | `text`, `fontSize`, `fillColor`, `printMode`, `onFinish`, `onProgress` |
| `<clip>`      | 裁剪     | 区域尺寸                                     |
| `<filter>`    | 滤镜     | filter 类型与参数                            |
| `<backdrop>`  | 背景滤镜 | filter 类型与参数                            |
| `<animation>` | 帧动画   | 动画配置                                     |
| `<video>`     | 视频     | `src`, `onEnded`, `onStateChange`            |

### 通用节点属性（`MoyuNodeAttributes`）

`MoyuNodeAttributes` = `MoyuListenerAttributes` + `NodeProps`（由 bindings 生成）+ `children`。

**来自 `NodeProps`（`bindings/NodeProps.ts`）：**

```typescript
type NodeProps = {
  label?: string;
  anchor?: [number, number];  // 0.0-1.0
  pivot?: [number, number];   // pixels
  x?: number; y?: number;
  scale?: number; scaleX?: number; scaleY?: number;
  rotation?: number;
  skew?: number; skewX?: number; skewY?: number;
  visible?: boolean;
  tint?: string;
  opacity?: number;
  interactive?: boolean;
  cursor?: MoyuCursor;
};
```

**来自 `MoyuListenerAttributes`（`declaration.ts`）：**

```typescript
interface MoyuListenerAttributes {
  onClick?: MoyuEventHandler<MouseEvent>;
  onMouseEnter?: MoyuEventHandler<MouseEvent>;
  onMouseLeave?: MoyuEventHandler<MouseEvent>;
  onMouseDown?: MoyuEventHandler<MouseEvent>;
  onMouseUp?: MoyuEventHandler<MouseEvent>;
  onMouseMove?: MoyuEventHandler<MouseEvent>;
  onKeyDown?: MoyuEventHandler<KeyboardEvent>;
  onKeyUp?: MoyuEventHandler<KeyboardEvent>;
  onKeyPress?: MoyuEventHandler<KeyboardEvent>;
  onTouchStart?: MoyuEventHandler<TouchEvent>;
  onTouchMove?: MoyuEventHandler<TouchEvent>;
  onTouchEnd?: MoyuEventHandler<TouchEvent>;
  onTouchCancel?: MoyuEventHandler<TouchEvent>;
}
```

注意：`declaration.ts` 当前并未暴露 `onWheel`，即使 wheel 事件会冲出 JS 层；如需需在节点上绑定 wheel，目前的做法是全局 `addEventListener('wheel', ...)`。这应当在未来得到改进，改进后本处描述应当更新。

---

## 开发规范

### 新增内置组件（需 Rust + kit 配合）

1. Rust 端：在 `crates/nodes` 添加节点类型（见根 AGENTS.md），用 `#[derive(TS)]` 标注 Props。
2. 重新生成 bindings：`yarn generate:bindings`。
3. 在 `declaration.ts` 中添加 JSX 声明：

```typescript
// declaration.ts
export type MoyuMyComponentAttributes = MyComponentProps & MoyuNodeAttributes;

declare namespace JSX {
  interface IntrinsicElements {
    mycomponent: DetailedMoyuProps<MoyuMyComponentAttributes>;
  }
}
```

4. 如需 spring 支持，在 `spring/primitives.ts` 中加入对应 `animated.mycomponent`。

### 新增全局事件类型

1. 在 `events/` 下新建事件类型文件（如果不是 bindings 自动生成的）。
2. 在 `events/globals.ts` / `events/listener.ts` 中处理事件分发。
3. 在 `events.ts` 中 re-export。

### 新增 hook

- 落在 `hooks/` 下单独文件，在 `hooks.ts` 中 re-export。
- 优先复用 `useStage` / `useScenario` / valtio snapshot 等既有能力。

### 修改 bindings

**绝对不要**直接编辑 `bindings/` 下的文件。流程：

1. 修改 Rust 侧的 `#[derive(TS)]` 结构体。
2. 在仓库根目录运行 `yarn generate:bindings`。
3. 检查 diff，确认改动符合预期。

---

## 环境兼容性

kit 设计为同时支持：

- **浏览器环境**：完整 Web API
- **QuickJS 环境**（原生平台）：受限 JS 运行时

QuickJS 环境的 polyfill / 类型垫片：

- `requestAnimationFrame` / `cancelAnimationFrame`（`events/globals.ts`，非浏览器环境下基于引擎 `animationframecallbackevent` 实现）
- `console` 与其它 Web 标准 API 类型声明（`runtime-globals.ts`）

**环境感知：**

编写代码时假设代码可能同时运行在两种环境；不要直接依赖 `window` / `document` 之类只在浏览器存在的全局。

---

## 构建

```bash
# 仓库根目录
yarn build               # 构建所有 packages
yarn generate:bindings   # 重新生成 bindings

# packages/kit 目录
yarn build               # 仅构建 kit（tsup）
yarn dev                 # watch 模式
```

`tsup` 配置输出双入口：ESM（`dist/lib.mjs`）和 CJS（`dist/lib.cjs`），同时提供 `jsx-runtime` / `jsx-dev-runtime` 子路径导出。

---

## 技术选型概览

- **React 19 + react-reconciler**：提供自定义渲染器基础，负责把 React 树映射到 Moyu 节点树。
- **react-spring**：提供 `animated.<element>`、spring host 和时间调度能力，用于声明式动画与交互动画。
- **valtio**：用于 Stage 的 skip / auto 响应式状态，以及与 sandbox / 运行时协作的轻量状态读取。
- **zod**：用于命令 schema、运行时解析和上层项目中的命令验证。

具体版本、导出格式和构建细节以 `package.json`、`tsup.config.ts` 与源码为准。

---

## 注意事项

1. **不支持 HTML 元素**：Reconciler 仅接受 Moyu JSX elements。
2. **事件冒泡**：遵循类似 DOM 的冒泡机制。
3. **属性命名**：JS 端使用 camelCase，与 Rust 端 snake_case 自动转换。
4. **类型安全**：充分利用 bindings 与 TS 类型系统。
5. **性能**：避免在渲染循环/事件回调中进行大量分配或同步计算。
6. **bindings 只读**：一切类型变更走 Rust + `yarn generate:bindings`。
7. **HMR 与 scenario**：`useScenario` 依赖 session key、refcount 和串行生命周期队列实现 HMR 弹性；相同 key 的瞬时 remount 会复用 live session，只有 key 变化或最终卸载时才会真正 `terminateStory`。
8. **CJS/ESM 分发**：改动打包配置前确认消费者（框架、SDK 用户）不会因运行时重复化（如 valtio）而失效。
