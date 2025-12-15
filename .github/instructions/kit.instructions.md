---
applyTo: '**'
---

# Moyu JavaScript SDK - Agent Coding Instructions

## 工程结构

`packages/` 目录是一个 monorepo，包含末语引擎的 JavaScript 部分：

| 包名              | 路径                 | 用途                                  |
| ----------------- | -------------------- | ------------------------------------- |
| `@momoyu-ink/kit` | `packages/kit`       | 核心 SDK，提供 React 渲染器和引擎 API |
| bunnymark         | `packages/bunnymark` | 性能基准测试（使用纯 Node API）       |
| gallery           | `packages/gallery`   | 示例展示，可作为用户参考代码          |

## Kit 包架构

### 核心模块

```
packages/kit/src/
├── lib.ts           # 主入口，导出所有公共 API
├── moyu.ts          # 引擎底层通信 API（私有）
├── node.ts          # Node 节点封装类
├── react.ts         # React Reconciler 实现
├── declaration.ts   # TypeScript 类型声明和 JSX 命名空间
├── events.ts        # 事件系统和全局事件监听
├── state.ts         # 内部状态管理
├── utils.ts         # 工具函数
├── events/          # 事件类型定义
│   ├── base.ts      # BubbleEvent 基础类型
│   ├── mouse.ts     # 鼠标事件
│   ├── touch.ts     # 触摸事件
│   ├── keyboard.ts  # 键盘事件
│   ├── node.ts      # 节点事件
│   ├── raf.ts       # requestAnimationFrame 事件
│   └── custom.ts    # 自定义事件
└── spring/          # react-spring 动画集成
    ├── index.ts
    ├── animated.ts  # Animated 组件类型
    └── primitives.ts
```

### 技术实现

#### 1. 自定义 React Reconciler

Kit 实现了完整的 React Reconciler，将 React 组件树映射到引擎节点树：

```typescript
// react.ts 核心逻辑
const hostConfig: HostConfig<...> = {
  createInstance(type, props) {
    // 创建引擎节点
    return Node.create(props.label ?? '', type, props);
  },
  appendChild(parent, child) {
    parent.addChild(child);
  },
  // ...更多方法
};
```

**重要**：不支持原生 HTML 元素，仅支持 Moyu 特有组件。

#### 2. 引擎通信层 (moyu.ts)

通过 `moyu` 全局对象与 Rust 引擎通信：

```typescript
// 底层命令
moyu.pushCommand(name, args, callback); // 通用命令
moyu.executeNodeCommand(nodeId, payload); // 节点命令
moyu.executePluginCommand(pluginName, payload); // 插件命令
```

**封装的 API**：

- `createInstance()` - 创建节点
- `destroyInstance()` - 销毁节点
- `addChild()` / `removeChild()` - 节点树操作
- `updateProps()` - 更新属性
- `executeNodeCommand()` / `executePluginCommand()` - 执行命令

#### 3. 事件系统

事件从 Rust 层通过 `__moyu_receive_event` 传递到 JS 层，支持：

- **冒泡事件**：MouseEvent, TouchEvent, KeyboardEvent
- **节点事件**：NodeEvent (创建、销毁)
- **自定义事件**：CustomEvent
- **全局事件**：通过 `addEventListener()` 监听

```typescript
// 使用全局事件监听
import { addEventListener } from '@momoyu-ink/kit';

addEventListener('keydown', (event: KeyboardEvent) => {
  // 处理键盘事件
});
```

#### 4. 动画系统

基于 `react-spring` 实现声明式动画：

```typescript
import { animated, useSpring } from '@momoyu-ink/kit';

function AnimatedSprite() {
  const styles = useSpring({ opacity: 1, from: { opacity: 0 } });
  return <animated.sprite style={styles} src="image.png" />;
}
```

## 内置组件

### JSX IntrinsicElements

| 组件          | 用途       | 关键属性                                     |
| ------------- | ---------- | -------------------------------------------- |
| `<container>` | 容器节点   | 基础变换属性                                 |
| `<sprite>`    | 图片精灵   | `src`, `area`, `mode`, `bounds`              |
| `<yuvsprite>` | YUV 视频帧 | `area`                                       |
| `<video>`     | 视频播放   | `src`, `autoplay`                            |
| `<text>`      | 文本渲染   | `text`, `fontSize`, `fillColor`, `printMode` |

### 通用节点属性 (MoyuNodeAttributes)

```typescript
interface MoyuNodeAttributes {
  label?: string; // 节点标签（调试用）
  x?: number; // X 坐标
  y?: number; // Y 坐标
  anchor?: [number, number]; // 锚点 (0.0-1.0)
  pivot?: [number, number]; // 旋转中心点 (像素)
  scale?: number; // 缩放
  rotation?: number; // 旋转角度
  visible?: boolean; // 可见性
  tint?: string; // 着色
  opacity?: number; // 透明度
  interactive?: boolean; // 是否响应交互
  cursor?: Cursor; // 鼠标样式
  // 事件处理器
  onClick?: MoyuEventHandler<MouseEvent>;
  onMouseDown?: MoyuEventHandler<MouseEvent>;
  // ...更多事件
}
```

## 开发规范

### 创建新组件

1. 在 Rust 层 (`crates/nodes`) 添加节点类型
2. 在 `declaration.ts` 添加属性类型和 JSX 声明：

```typescript
// declaration.ts
export interface MoyuMyComponentAttribute extends MoyuNodeAttributes {
  myProp?: string;
}

// 在 JSX.IntrinsicElements 中添加
interface IntrinsicElements {
  // ...existing
  mycomponent: DetailedMoyuProps<MoyuMyComponentAttribute>;
}
```

3. 在 `spring/primitives.ts` 添加 animated 支持（如需要）

### 添加全局事件类型

1. 在 `events/` 下创建事件类型文件
2. 在 `events.ts` 中处理事件分发
3. 导出类型供用户使用

### Gallery 示例规范

Gallery 包的代码将被用户参考和复制，请确保：

- 代码清晰、简洁、结构良好
- 添加必要的注释说明
- 使用最佳实践和惯用写法
- 展示组件的典型用法

## 环境兼容性

Kit 设计为同时支持：

- **浏览器环境**：完整 Web API 支持
- **QuickJS 环境**：受限的 JS 运行时（原生平台）

QuickJS 环境的 polyfill（在 `events.ts` 中）：

- `requestAnimationFrame` / `cancelAnimationFrame`

## 依赖说明

| 依赖             | 版本    | 用途           |
| ---------------- | ------- | -------------- |
| react            | ^18.2.0 | UI 框架 (peer) |
| react-reconciler | ^0.29.2 | 自定义渲染器   |
| @react-spring    | ^9.7.3  | 动画系统       |

## 注意事项

1. **不支持 HTML 元素**：React Reconciler 仅处理 Moyu 组件
2. **事件冒泡**：遵循类似 DOM 的事件冒泡机制
3. **属性命名**：使用 camelCase，与 Rust 层的 snake_case 自动转换
4. **类型安全**：充分利用 TypeScript 类型系统
5. **性能考虑**：避免在渲染循环中进行大量计算
