# Moyu Gallery — Agent 指南

本文档面向 AI coding agent 和开发者，描述 `@momoyu-ink/gallery` 的定位、结构与开发约定。

> 引擎核心与通用规范见仓库根 [AGENTS.md](../../AGENTS.md)。Kit API 与运行时约束见 [packages/kit/AGENTS.md](../kit/AGENTS.md)。本文件只覆盖 Gallery 包内约定。

---

## 概述

Moyu Gallery 是末语引擎节点与 `@momoyu-ink/kit` 组件的交互式展示应用，同时承担：

- 开发时手动验证引擎节点、Kit 组件与布局行为；
- 对外展示 Sprite、Text、Filter、Backdrop、动画、Shader、布局和封装组件；
- 为 Kit API 变更提供消费端类型检查与构建验证。

Gallery 使用 React + TypeScript + Vite，运行于 Moyu 自定义 React Reconciler。JSX 只能使用 Moyu intrinsic elements（如 `<container>`、`<sprite>`、`<text>`），不能使用 HTML 元素或 `react-dom`。

---

## 目录结构

```
packages/gallery/
├── src/
│   ├── index.tsx                # ready 事件与应用入口
│   ├── gallery.tsx              # 应用骨架、导航、外观切换、舞台缩放与内容滚动
│   ├── theme.ts                 # 主题公共入口（活单例 + applyGalleryMode）
│   ├── theme/                   # 主题配置：tokens.json + 调色板/sprite 构造
│   ├── components/
│   │   └── chrome.tsx           # Panel、DemoChip、SectionTabs 等共享展示组件
│   └── pages/                   # 各能力展示页
├── scripts/
│   └── generate-assets.mjs      # 零依赖 PNG 资产生成脚本
├── assets/
│   ├── images/                  # 脚本生成的 UI 与展示贴图
│   ├── cursors/                 # 脚本生成的鼠标指针图片
│   ├── generated/               # 其它生成资源
│   └── fonts/                   # Gallery 字体
├── docs/                        # Gallery 迭代与设计记录
├── README.md                    # 使用说明
├── index.json                   # Moyu 项目配置
├── index.js                     # Vite 开发入口，保持 index.json 的入口协议
└── vite.config.ts               # 构建与开发服务器配置
```

---

## 开发约定

### 页面与组件

- `src/gallery.tsx` 只负责应用骨架、页面导航和共享滚动能力；具体展示内容放在 `src/pages/`。
- 页面间重复的展示框架优先复用 `src/components/chrome.tsx`，颜色和 sprite 配置统一从 `src/theme.ts` 获取。
- 页面既是展示页面也是手动测试用例。新增能力时应给出可观察的输入、状态或预期结果，避免只放静态说明文字。
- Gallery 依赖工作区内的 `@momoyu-ink/kit`。修改 Kit 公共 API 后，应更新对应 Gallery 用例并运行 Gallery 类型检查与构建。
- Gallery 中需要直接、简明地体现引擎提供的接口，而不是通过复杂的封装或间接调用。避免在 Gallery 中隐藏引擎能力或使用不必要的中间层。

### 运行时兼容

- 代码需要兼容浏览器与 Native QuickJS，不要依赖 `window`、`document` 或其它仅浏览器存在的 API。
- 使用 Kit 的 `createRoot()`、事件和节点 API，不引入 `react-dom`。
- 资源路径相对于 Gallery 项目资产目录，例如 `images/button.png` 对应 `assets/images/button.png`。

### 主题与资产

- 颜色只在 `src/theme/tokens.json` 写一次：运行时主题与 `scripts/generate-assets.mjs` 共用它，新增颜色或圆角先改这里。页面中不要硬编码控件颜色。
- `src/theme.ts` 是主题公共入口（页面统一 `import ... from '../theme'`），`src/theme/` 放 tokens 与调色板/sprite 构造。它导出的 `COLOR` / `TEXT` / 各 sprite 是**活单例**，`applyGalleryMode()` 就地改写它们以切换浅色/深色。页面模块顶层的常量若复制了其中的值，切换后会冻结在旧外观上，必须用 getter 或 `sizedButton()` 这类实时视图。
- 控件与面板资产按明暗两套烘焙在 `assets/images/light/` 与 `assets/images/dark/`。主题不使用描边，层级由"页面底 → 卡片面 → 控件填充"三级明度差表达，单张中性纹理乘一个 tint 表达不了两套外观。只有 `pixel.png` / `chip.png` / `ring.png` 保持中性并在运行时 `tint`。
- `assets/images/` 下由 `scripts/generate-assets.mjs` 生成的文件禁止手工修改。修改生成逻辑后重新运行 `generate:assets`，并检查生成图片。
- 九宫格资产必须在主题配置中同时提供 `mode: 'nineslice'` 与正确的 `bounds`；拉伸后的目标尺寸由调用方或主题提供。

---

## 常用命令

在仓库根目录运行：

```bash
yarn workspace @momoyu-ink/gallery dev
yarn workspace @momoyu-ink/gallery typecheck
yarn workspace @momoyu-ink/gallery build
yarn workspace @momoyu-ink/gallery generate:assets
```

- `dev`：启动 Vite 开发服务器，默认端口 `6023`。持续运行的开发服务器由用户启动，agent 不要自行启动。
- `typecheck`：运行 TypeScript 类型检查。
- `build`：运行生产构建，验证 Gallery 作为 Kit 消费端可以正确打包。
- `generate:assets`：重新生成全部 UI 图片；运行后必须检查资产 diff，避免把无关生成变化混入当前任务。

开发服务器启动后，可用 Moyu 引擎加载 `http://localhost:6023/index.json` 进行运行时手动验证。

---

## 验证要求

1. 修改 TypeScript 后运行 `typecheck`；涉及运行时或构建配置时同时运行 `build`。
2. 修改生成脚本或主题资产配置后运行 `generate:assets`，检查对应 PNG，并再次运行 `typecheck` 与 `build`。
3. 修改交互、布局、滤镜、动画或 Shader 行为时，在引擎中打开对应页面进行手动验证；自动检查不能替代运行时视觉与交互检查。
4. 不运行全仓格式化，不整理与当前任务无关的 Gallery 文件或生成资产。

---

## 注意事项

1. Gallery 不是 DOM 应用，只能使用 Kit 支持的 Moyu JSX 元素。
2. `src/theme.ts` 与资产生成脚本需要保持一致，避免重复 tint 或把固定颜色误烘焙进可复用资源。
3. `generate:assets` 会重写全部生成图片；提交前应核对实际变化范围。
4. 页面内的注释使用英文；面向用户的展示文案当前以中文为主，技术名词可保留英文。
