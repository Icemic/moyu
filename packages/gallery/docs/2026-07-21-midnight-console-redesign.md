# Gallery 重新设计迭代文档 — Midnight Console

日期：2026-07-21
状态：已实施

## 1. 背景

Gallery 项目源自调试用的 `layout-test.tsx` 测试页，目标是一个兼具**开发时手动测试**与**对外组件能力展示**的小应用（类似 Storybook）。第一版实现存在三个主要问题：

1. **视觉风格与 layout-test 差异过大**：layout-test 的"深色面板 + 金色标题 + 五色边框色块"视觉语言被完全丢弃，替换为裸文字 + 霓虹色文字，没有继承任何原有气质。
2. **缺乏设计感**：无背景、无面板分区、无视觉层级；控件贴图为近黑色（RGB 17）配灰色描边，对比度极低。
3. **功能展示安排不合理**：布局页的测试项退化为纯文字（丢失了可视化色块）；Filter 页只有稀疏的文字样本；各页信息密度和排版各自为政。

### 问题根源（技术诊断）

- 引擎的 `tint` 是**乘法**（`texel × tint`，见 `crates/nodes/src/renderer/shaders/default.wgsl`）。layout-test 的彩色边框效果依赖于贴图中的**浅灰描边像素**乘 tint；而第一版 gallery 大量展示内容根本不使用贴图，只用文字颜色，因此全盘丢失了风格。
- `tint` 定义在 `NodeProps`，**不向子节点级联**（只有 `opacity` 级联）；且 kit 的 `SelectListProps` / `SliderTrackProps` / `SliderThumbProps` 仅扩展 `SpriteProps`，**不接受 tint**。这三类资产必须预烘焙颜色。
- 旧资产（`selection.png` 等）主体为近黑色，乘法 tint 后几乎不可见，无法支撑任何配色体系。

## 2. 决策记录

通过与需求方确认的三个关键决策：

| 决策点 | 结论 | 备选 |
|---|---|---|
| 视觉风格 | **A · 深夜控制台**（layout-test 风格的正统演进） | B 蓝图工程 / C 纸面浅色 / D 霓虹玻璃 |
| 界面语言 | **中文为主**，技术术语保留英文 | 中英混排 / 全英文 |
| 页面结构 | **保持 8 页**（Spring 动画与 Shader 转场各自独立） | 合并为 7 页 |

风格 A 的设计令牌从 layout-test 提炼并精化：深海军蓝底、暗色面板 + 描边、金色标题、原始五色（`#4f75c9` 蓝 / `#5f9b72` 绿 / `#a66b75` 红 / `#7d68b5` 紫 / `#b18448` 橙）。

## 3. 设计系统

### 3.1 资产管线（`scripts/generate-assets.mjs`）

所有 UI 贴图由零依赖 Node 脚本生成（内置 zlib + 手写 CRC32/PNG 编码，4× 超采样抗锯齿），替代原 PowerShell 脚本。重新生成：

```bash
yarn workspace @momoyu-ink/gallery generate:assets
```

资产分两类：

| 类别 | 资产 | 着色方式 |
|---|---|---|
| 中性可 tint | `panel` / `chip` / `button*` / `checkbox*` / `dropdown`（trigger）/ `dropdown_listitem*` / `pixel` | 灰度底（填充灰 + 白色描边），运行时由 `theme.ts` 的 tint 上色：白色像素精确呈现 tint 色，灰色像素呈现同色相暗色 |
| 预烘焙 | `dropdown_list` / `slider_track*` / `slider_handle*` | 对应 kit props 类型不含 tint，生成时直接烘焙控件色 `#54688c` |

其余资产：`bg.png`（1920×1080 深蓝渐变 + 暗角 + 顶部微光，烘焙）、`sample.png`（四象限饱和色 + 色相渐变条，供 Sprite/Filter 演示）、`cursor.apng` 与 `generated/mask-rule-horizontal.png`（沿用）。

### 3.2 设计令牌（`src/theme.ts`）

所有颜色决策的唯一来源：

- `COLOR.panelTint #2c3a52` — 面板描边色（填充自动成为其暗色 `#161d29`）
- `COLOR.controlTint #54688c` / `controlTintActive #3d5a80` — 控件常态 / 选中
- `COLOR.panelTitle #e8c97a` — 金色标题（精化自 layout-test 的 `#f7d98b`）
- `COLOR.text #dbe4f3` / `caption #8fa0bd` / `dim #5c6a84` — 文字层级
- `COLOR.accent #e8c97a` — 选中态与强调
- `ITEM_COLORS` — 原始五色展示色块
- `TEXT.*` — 文字预设；`PANEL_SPRITE` / `chipSprite(color)` / `BUTTON_SPRITE` / `SELECT_*` / `SLIDER_*` / `ZONE_SPRITE` / `PIXEL_SPRITE` — 精灵预设

### 3.3 共享组件（`src/components/chrome.tsx`）

- `Panel` — 所有展示内容的基本框架单元：九宫格面板 + 金色标题（内容区约定从 `x=20, y=56` 开始）+ 底部 `note` 说明/预期文字（继承 layout-test"预期结果"的测试传统）
- `DemoChip` — 带彩色描边与居中文本的演示色块（恢复 layout-test 的 TestItem 视觉）
- `SectionTabs` — 页内分段切换（布局页使用）

### 3.4 应用骨架（`src/gallery.tsx`）

- 舞台缩放：以 1920×1080 设计分辨率居中缩放适配实际窗口（继承 layout-test 的做法）
- 左侧：品牌区（末语画廊 / MOYU GALLERY）+ 面板包裹的 ScrollView 编号导航（01–08，选中项金色文字 + 金色指示条）+ 底部许可信息
- 右侧：页头（页面标题 + 描述 + 分隔线）+ 1504×912 内容区，页面内容均设计为免滚动完整呈现

## 4. 页面结构

| # | 页面 | 内容 |
|---|---|---|
| 01 | 基础组件 | Text 样式 / Container 交互变换 / Sprite（area + nineslice）/ Animation（APNG）/ Clip |
| 02 | 封装组件 | Button（计数、禁用、lockOn、对齐）/ Checkbox / Slider / Select / ScrollView |
| 03 | Filter 滤镜 | 原始对照 + 9 种滤镜的 2×5 网格，样本为 `sample.png` + 文字 |
| 04 | Backdrop 背景滤镜 | 左控制台（开关、模糊半径、饱和度）+ 右演示场景（z 序：背景文字 → backdrop → 前景清晰内容） |
| 05 | Spring 动画 | useSpring 属性动画 / useTransition 进出场 / 淡入淡出辅助 |
| 06 | Shader 转场 | 控制台（效果、时长、播放、状态）+ 双通道转场画面（prepare→perform 状态机） |
| 07 | 自定义 Shader | Raw WGSL 预设（Color shift / Wave / Scan）+ 参数槽 + manual/auto 时间控制 |
| 08 | 布局 | 四个子页（分段切换）：基础与嵌套 / 对齐交互 / 变换与测量 / 动态重排 —— 完整恢复 layout-test 的 9 项测试 |

## 5. 验证

- `yarn workspace @momoyu-ink/gallery typecheck` — 通过
- `yarn workspace @momoyu-ink/gallery build` — 通过
- 视觉效果由需求方运行确认（`yarn workspace @momoyu-ink/gallery dev`）

## 6. 已知限制与后续方向

- 字重缺失：仅 SourceHanSansSC-Regular 一款字体，标题层级只能靠字号与颜色区分；后续可考虑引入 Bold 字重。
- `SelectListProps` / `SliderTrackProps` / `SliderThumbProps` 不含 tint（kit 类型限制），相关资产颜色变更需重新运行 `generate:assets`；若未来 kit 为这些 props 补充 tint，可改回中性资产。
- 面板圆角烘焙在贴图中（半径 8/6px），超大尺寸拉伸下圆角保持不变（九宫格），但极小尺寸（< 32px）不建议使用 panel/chip 资产。
