# Moyu Gallery 末语画廊

末语引擎节点与 `@momoyu-ink/kit` 组件的交互式画廊。既是开发时的手动测试台，也是对外的能力展示应用。

视觉设计为 **Midnight Console**（深夜控制台）：深海军蓝底、暗色描边面板、金色标题、五色演示色块。设计令牌统一收敛在 `src/theme.ts`，迭代文档见 `docs/2026-07-21-midnight-console-redesign.md`。

## 开发

在仓库根目录执行：

- `yarn workspace @momoyu-ink/gallery dev` — rspack 开发服务器（端口 6023）
- `yarn workspace @momoyu-ink/gallery typecheck` — 类型检查
- `yarn workspace @momoyu-ink/gallery build` — 构建
- `yarn workspace @momoyu-ink/gallery generate:assets` — 重新生成全部 UI 贴图资产

应用只使用 Moyu intrinsic JSX 元素与 Kit 自定义渲染器，不包含 DOM 元素或 `react-dom`。

## 页面

- 01 基础组件 — Sprite、Text、Clip、Animation 与通用节点属性
- 02 封装组件 — Button、Checkbox、Select、Slider 与 ScrollView
- 03 Filter 滤镜 — 原始对照 + 9 种滤镜样本
- 04 Backdrop 背景滤镜 — 交互式背景捕获演示
- 05 Spring 动画 — useSpring 与 useTransition
- 06 Shader 转场 — 双通道 GPU 转场状态机
- 07 自定义 Shader — Raw WGSL 与参数槽
- 08 布局 — VBox、HBox、测量、对齐与动态重排（四个子页）

## 资产

`assets/images/` 下几乎所有 PNG 都由 `scripts/generate-assets.mjs`（零依赖）生成，请勿手工编辑。其中中性灰度资产在运行时通过 `tint` 上色；`dropdown_list` 与 `slider_*` 因 kit props 不含 tint 而为预烘焙色。

## 许可

MPL-2.0，见 `LICENSE.txt`。
