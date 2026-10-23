# Moyu Gallery

末语引擎节点与 `@momoyu-ink/kit` 组件的交互式组件展示。既是开发时的手动测试台，也是对外的能力展示应用。

## 开发

在仓库根目录执行：

- `yarn workspace @momoyu-ink/gallery dev` — Vite 开发服务器（端口 6023）
- `yarn workspace @momoyu-ink/gallery i18n:extract` — 提取并更新中、英、日消息目录
- `yarn workspace @momoyu-ink/gallery typecheck` — 类型检查
- `yarn workspace @momoyu-ink/gallery build` — 构建
- `yarn workspace @momoyu-ink/gallery generate:assets` — 重新生成全部 UI 贴图资产

应用只使用 Moyu intrinsic JSX 元素与 Kit 自定义渲染器，不包含 DOM 元素或 `react-dom`。

## 国际化

Gallery 使用 Lingui 6，主语言为中文，同时维护 `zh`、`en` 和 `ja` catalog。新增或修改界面文案后运行 `i18n:extract` 更新 `src/locales/{locale}/messages.po`。Vite 在构建时通过 Lingui 插件编译 catalog，不需要单独执行 compile。

## GitHub Pages

`main` 分支的 Release workflow 会在全部引擎平台构建完成后：

1. 构建并类型检查 Gallery；
2. 下载同次运行产生的 `web-universal-dev` 引擎 artifact；
3. 组合 Web 引擎、Gallery bundle、`index.json`、`index.html` 和 `assets/`；
4. 部署组合产物到 GitHub Pages。

仓库需要在 GitHub Settings 中启用 Actions 作为 Pages source，并创建名为 `github-pages` 的 environment。tag 发布不会覆盖 Gallery Pages。

## 页面

- 01 基础组件 — Sprite、Text、Clip、Animation 与通用节点属性
- 02 布局 — VBox、HBox、测量、对齐与动态重排（四个子页）
- 03 封装组件 — Button、Checkbox、Radio、Select、Slider 与 ScrollView
- 04 Filter 滤镜 — 原始对照 + 9 种滤镜样本
- 05 Backdrop 背景滤镜 — 交互式背景捕获演示
- 06 Spring 动画 — useSpring 与 useTransition
- 07 Shader 转场 — 双通道 GPU 转场状态机
- 08 自定义 Shader — Raw WGSL 与参数槽
- Live2D（仅桌面端）— 模型、动作、表情、参数、部件不透明度、命中测试与遮罩渲染

## 资产

`assets/images/` 下几乎所有 PNG 都由 `scripts/generate-assets.mjs`（零依赖）生成，请勿手工编辑。其中中性灰度资产（包括 `slider_*`）在运行时通过 `tint` 上色；仅 `dropdown_list` 因 kit props 不含 tint 而为预烘焙色。

### Live2D（本机放置）

Live2D 页面依赖两份不入库的本地内容，需要自行准备：

1. 模型：在 `assets/live2d/` 下按 `<名字>/<名字>.model3.json` 放置，页面内置的条目为 `Hiyori`、`Hiyori2`、`Mao`、`Haru`（即 `assets/live2d/Hiyori/Hiyori.model3.json` 等）。
2. 插件：把 `moyu_live2d_plugin`（Windows 为 `.dll`，macOS 为 `.dylib`，其它为 `.so`）与 `Live2DCubismCore` 一起放到引擎可执行文件旁边；也可以用 `MOYU_LIVE2D_PLUGIN` 环境变量指向插件路径。

两者都已被 `packages/gallery/.gitignore` 排除。未准备时页面不崩溃，而是显示错误信息与这份放置说明；Web 引擎没有动态库，同样按降级处理。

## 许可

MPL-2.0，见 `LICENSE.txt`。
