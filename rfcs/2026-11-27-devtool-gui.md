# RFC：`moyu-devtool` 图形界面设计（GPUI）

- **状态**：已定稿
- **日期**：2026-11-27（修订于 2026-11-28）
- **作者**：末语项目组
- **适用范围**：`crates/devtool/src/gui/`、`crates/devtool/assets/`、`.github/workflows/`
- **相关 RFC**：[基于 Rust 的统一开发工具链（`moyu-devtool`）](./2026-11-26-moyu-devtool.md)

## 1. 摘要

本文定义 `moyu-devtool` 图形界面壳层（`crates/devtool/src/gui/`）的渲染选型、视觉与主题体系、图标与原子组件库、单窗口导航结构、四大核心工作区以及进程退出生命周期规范。底层服务、领域模型、CLI 契约与打包分发规范见 [总 RFC](./2026-11-26-moyu-devtool.md)。

## 2. 技术选型与构建策略

### 2.1 GPUI 依赖锁定

- 直接通过 Git 依赖引用 `zed-industries/zed` 中的 `gpui` 与 `gpui_platform`，**锁定具体 commit `rev`**（当前为 Zed `v0.233.10` / `350f3387`），不追踪浮动分支。
- 升级 `gpui` 采用按需人工提升 `rev` 并集中适配的方式，避免上游 API 演进破坏日常构建。

### 2.2 Feature 隔离与 CI 依赖

- `crates/devtool/Cargo.toml` 定义 `gui` feature 并设为默认启用（`default = ["gui"]`），控制 `gpui`、`gpui_platform` 与 `unicode-segmentation` 依赖。
- 纯 CLI 快速编译验证使用 `cargo build -p moyu_devtool --no-default-features`。
- Linux CI（`.github/workflows/build.yml`、`devtool-release.yml`）安装 GPUI 所需的系统图形与输入库：`libxkbcommon-dev`、`libxkbcommon-x11-dev`、`libwayland-dev`、`libvulkan-dev`、`libxcb1-dev`、`libx11-xcb-dev`、`libfontconfig1-dev`。

## 3. 视觉语言、主题、图标与国际化

### 3.1 配色体系（对齐 FishFlow）

所有主题共用同一套布局与尺寸，支持 4 套内置配色（`Light`、`Dark`、`Solarized Light`、`One Dark`）及 `System`（跟随操作系统明暗外观映射为 `Light` / `Dark`），偏好持久化至 `~/.moyu/devtool.json`。

| 语义 Token | Light | Dark | Solarized Light | One Dark | 用途 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `background` | `#FFFFFF` | `#0A0A0A` | `#FDF6E3` | `#282C34` | 窗口主底色、内容区底色 |
| `foreground` | `#0A0A0A` | `#FAFAFA` | `#657B83` | `#ABB2BF` | 主文案色 |
| `card` / `popover` | `#FFFFFF` | `#171717` | `#EEE8D5` / `#FDF6E3` | `#21252B` | 面板、浮层菜单底色 |
| `muted` / `meta` | `#EBEBEB` / `#5A5A5A` | `#222222` / `#B5B5B5` | `#EEE8D5` / `#7F8E91` | `#2C313C` / `#8E94A0` | 顶栏底色、弱提示文案 |
| `primary` / `foreground` | `#171717` / `#FAFAFA` | `#E5E5E5` / `#171717` | `#586E75` / `#FDF6E3` | `#404754` / `#F8FAFD` | 主操作按钮、激活选项 |
| `secondary` / `accent` | `#E8E8E8` / `#E5E5E5` | `#262626` / `#2A2A2A` | `#E6DFCB` / `#E3DCC6` | `#30333D` / `#323842` | 次级按钮、悬停/选中态 |
| `destructive` | `#DC2626` | `#EF4444` | `#DC322F` | `#C24038` | 停止运行、错误与危险操作 |
| `rule` / `input` / `ring` | `#D8D8D8` / `#DCDCDC` / `#737373` | `#2C2C2C` / `#333333` / `#737373` | `#D6CEB8` / `#D6CEB8` / `#268BD2` | `#323741` / `#1D1F23` / `#528BFF` | 分割线、输入框边框、焦点环 |
| `success` / `warning` / `info` | `#10B981` / `#CA8A04` / `#7C3AED` | `#10B981` / `#EAB308` / `#8B5CF6` | `#859900` / `#B58900` / `#268BD2` | `#98C379` / `#D19A66` / `#61AFEF` | 状态指示点、日志级别 |

### 3.2 尺寸与排版规范

- 逻辑像素尺寸对齐 FishFlow：基础圆角 `4px`，常规按钮高 `32px`（左右内边距 `11px`），紧凑按钮高 `26px`（左右内边距 `9px`），输入框与下拉框高 `30px`，标题栏高 `32px`，Windows/Linux 窗控按钮宽 `46px`，顶栏高 `36px`，底部状态栏高 `28px`。
- 字体沿用各操作系统原生无衬线与中文回退字体，不随二进制内嵌字体文件。

### 3.3 矢量图标系统（内嵌 Lucide SVG）

- 禁止使用原始 Unicode 符号或手绘 `div` 充当界面图标（避免字体基线与尺寸不一致）。
- 在 `crates/devtool/assets/icons/` 内嵌所需官方 Lucide SVG（`24×24` viewBox、`stroke-width="2"`），通过 `src/gui/ui/icon.rs` 的 `DevtoolAssets`（实现 `gpui::AssetSource`）在编译期打包，借助 GPUI 内置 `gpui::svg()`（`resvg`）按精确逻辑像素尺寸渲染并自动继承主题着色，零新增 Rust 依赖。

### 3.4 国际化（`src/gui/i18n.rs`）

- 全部可见界面文案均通过 `I18n` 统一管理，完整支持 `简体中文`（默认）与 `English` 实时切换，禁止在视图层硬编码界面文案。

## 4. 自建原子组件库（`src/gui/ui/`）

不引入第三方 GPUI 组件库，避免组件库锁定 `gpui` 版本升级。自建组件按职责拆分：

| 模块 | 职责与关键约束 |
| :--- | :--- |
| `theme.rs` | 全局 `Theme`（`gpui::Global`），提供 4 套主题与 `WindowAppearance` 解析 |
| `icon.rs` | `IconName` 枚举、`DevtoolAssets` 资源源与 `icon(name, size_px, color)` 渲染函数 |
| `button.rs` | 正交拆分 `ButtonVariant`（`Primary` / `Secondary` / `Outline` / `Ghost` / `Danger`）与 `ButtonSize`（`Sm = 26px` / `Md = 32px`），保证同组切换按钮尺寸严格一致；提供 `icon_button` |
| `dropdown.rs` | 通用 `SelectDropdown` 与 `popover_container` / `menu_item` / `menu_section_label` / `menu_divider`；**所有触发器与菜单项在 `MouseDown` 拦截事件冒泡**，确保展开态再次点击稳定收起；浮层通过 `gpui::deferred` 置顶渲染 |
| `card.rs` | `panel`、`muted_panel`、`section_heading`、`detail_row`、`status_dot`、`pulsing_status_dot`、`chip` |
| `motion.rs` | 基于 GPUI 内置 `AnimationExt` 的微动效原语（弹窗淡入上浮、下拉浮层滑入、抽屉高度展开、状态呼吸点），零第三方动效库依赖 |
| `progress.rs` | 细条水平进度条，配合任务阶段与字节统计展示 |
| `switch.rs` | `36×20px` 胶囊式布尔开关（圆形滑块 + 主题轨道着色 + 按压反馈） |
| `log_list.rs` | 基于 `gpui::uniform_list` 的虚拟滚动日志列表，支撑底部「任务输出」 |
| `text_input.rs` | 基于 `gpui::EntityInputHandler` 的单行文本输入框，支持选区、剪贴板、掩码（密码输入）与**中文 IME 组合输入** |

## 5. 单窗口外壳与导航架构

应用采用单窗口工作区（`src/gui/mod.rs`），后台持有独立 `tokio::runtime::Runtime` 执行全部异步服务，主线程通过 `cx.spawn` 消费事件通道更新状态并触发重绘。

```text
┌─────────────────────────────────────────────────────────────────────────┐
│ [Icon] 末语开发工具                            [─] [□/❐] [×]  (32px)    │  ← 第 1 行：自绘标题栏
├─────────────────────────────────────────────────────────────────────────┤
│ [📁 工程名 ▾]  [开发] [调试] [引擎] [打包]                 [⚙ 设置]     │  ← 第 2 行：工程切换 + 一级页签 + 设置 (36px)
├─────────────────────────────────────────────────────────────────────────┤
│                     （任务进行时显示进度条与取消按钮）                     │
│                                                                         │
│                            当前页签主工作区                              │
│                                                                         │
├─────────────────────────────────────────────────────────────────────────┤
│                     （可折叠「任务输出」抽屉 150px）                      │
├─────────────────────────────────────────────────────────────────────────┤
│ ● 状态消息                                            [任务与输出 ▴]    │  ← 底部状态栏 (28px)
└─────────────────────────────────────────────────────────────────────────┘
```

### 5.1 自绘标题栏平台规范

- 窗口启用 `TitlebarOptions { appears_transparent: true, ... }`。
- **Windows**：标题区与空白拖动区声明 `WindowControlArea::Drag`（映射 `HTCAPTION`，由系统原生处理拖动与双击最大化/还原，**禁止额外叠加手动双击 `zoom_window()`**）；最小化 / 最大化还原 / 关闭按钮声明 `WindowControlArea::Min / Max / Close`（映射 `HTMINBUTTON` / `HTMAXBUTTON` / `HTCLOSE`，原生支持最大化还原切换与 Windows 11 贴靠布局，绕过 `gpui` Windows 侧 `zoom_window()` 仅最大化不还原的行为）。
- **Linux**：标题栏按钮保留 `on_click` 兜底调用 `minimize_window` / `zoom_window` / `remove_window`。
- **macOS**：左侧预留原生交通灯区域（全屏态自动收起），应用图标与标题居中显示。

### 5.2 顶栏与浮层交互

- 顶栏底部分割线绝对定位在页签下方，激活页签以 `theme.background` 背景覆盖底线，与主工作区无缝衔接。
- **工程下拉**：展示当前工程与最近工程列表（含路径、当前勾选，非当前项右侧提供红色 `×` 一键移除且无需二次确认）；点击不存在或缺少 `index.json` 的工程时给出提示并静默从列表移除。底部提供「新建工程」（弹窗仅填写工程名称与父目录，无脚手架选择；提交后异步拉取 `DeepSpaceMill/framework` 最新 `main.zip`、按需自动安装托管 Node、安装依赖并下载激活最新引擎，完成后自动打开）和「打开已有工程」（调用 GPUI 原生目录选择器 `cx.prompt_for_paths`）。
- **设置弹窗**：仅保留「主题」与「界面语言」两项 `SelectDropdown` 偏好，即时生效并写入 `~/.moyu/devtool.json`。

## 6. 四大一级工作区

### 6.1 「开发」页（主工作区，`views/development.rs`）

- **页首**：工程窗口标题、工程路径、紧凑「打开文件夹」按钮，以及右侧两个分段按钮：
  1. **外部工具启动**：下拉按「编辑器 / 终端 / AI 工具」分组列出本机探测结果，主按钮按所选类型切换图标并直达当前工程目录，偏好持久化至 `preferredEditor`；无可用工具时禁用主按钮，菜单底部支持「重新检测」。
  2. **运行预览**：下拉切换「桌面运行 / 浏览器运行」，主按钮启动或停止运行；浏览器运行启动后自动唤起系统默认浏览器打开调试入口页。**与「调试」页共享同一 `DebugSession` 运行态**。
- **左列**：
  - **运行时配置（`index.json`）**：只读态展示窗口标题、应用名、窗口尺寸、舞台尺寸与框架摘要；点击「编辑」打开宽版可滚动「运行时配置」弹窗（`views/config_dialogs.rs`），按「窗口与基础」「多分辨率资源」「特性与平台（含 MSAA/Mipmaps/手柄/启动页/FPS/Steam/光标）」「▸ 高级渲染与入口（默认折叠）」4 个分组提供全部 25 项字段编辑（`fontFile` 与 `cursor` 均为可视化列表行编辑，自动按内容极简序列化为字符串或数组/对象，无需手写 JSON）。
  - **多分辨率资源**：展示当前工程检查结论并提供「重新检查」。
- **右列**：
  - **开发环境**：展示 Node.js 版本、路径、来源（本地托管优先于系统）、工程包管理器、`jiti` 状态与 **Dev Server 状态及启停按钮**（运行中展示实际端口 `localhost:<port>`），底部提供「安装/重装依赖」与「安装/卸载 Node LTS」。点击「桌面运行 / 浏览器运行」时若 Dev Server 未启动，将由 devtool 自动分配空闲端口拉起并等待就绪后再启动引擎（游戏关闭时不自动关闭 Dev Server）。
  - **Schema**：提供「剧情命令（`commands.schema.json`）」与「游戏 UI（`ui.schema.json`）」两个独立生成按钮。

### 6.2 「引擎」页（`views/engine.rs`）

- 页首提供「更新至最新」与「下载版本」（弹出下载对话框，自动拉取远端清单，通过 `SelectDropdown` 选择渠道/版本并勾选多平台下载）。
- 正文双栏：左栏展示当前激活版本、渠道/平台标签、CDN 地址与「配置」按钮（弹窗编辑 `moyu.json` 的 `engine.channel` 及折叠的 `cdnUrl` / `version` 高级源配置）；右栏展示本地已下载版本列表并支持一键切换激活。

### 6.3 「打包」页（`views/pack.rs`）

- 页首提供「开始打包」。
- 正文双栏：左栏为 2×2 导出目标卡片（桌面 / Web / Android / 框架）；右栏动态展示目标对应选项（桌面平台 `SelectDropdown`、Android 格式 `SelectDropdown` / 发布签名密码掩码输入框 / 「Android 工程配置」弹窗编辑 `moyu.json` 的 `android.*` 全部字段、ZIP 压缩包开关、视频解码插件开关与输出目录 `.moyu/release`）。

### 6.4 「调试」页（`views/debug.rs`）

- **统一运行态**：与「开发」页运行预览共用 `DebugSession` 与模式状态，默认窗口模式（`debug_headless = false`）；从任一入口启动后，两端按钮与状态即时同步。
- **会话栏**：通过 `SelectDropdown` 选择模式（桌面引擎 / Web 引擎 / 附加外部进程），桌面模式可切换「无窗口模式 / 显示窗口」，展示连接状态与可复制的 `MOYU_ENGINE_DEBUG_WS` / Web 地址，运行中提供「刷新节点树」「舞台截图」「拉取引擎日志」。
- **检查器主区**：
  - 左侧：可折叠、支持滚轮滚动（`overflow_y_scroll`）的舞台节点树。
  - 右侧：JS `eval` 输入与可滚动结果区、可滚动节点属性 JSON 视图、按需展示的 WebP 舞台截图预览（`gpui::Image::from_bytes`）与保存至 `.moyu/screenshots/`，以及支持级别过滤（全部 / ERROR / WARN / INFO / DEBUG）与滚轮滚动的引擎日志流。

## 7. 退出生命周期与资源回收

- 窗口创建时同时注册 `window.on_window_should_close` 与 `cx.on_app_quit`，在窗口关闭（标题栏关闭、Alt+F4、任务栏关闭）或应用退出前同步调用 `stop_debug_session_sync`：取消活动任务、终止运行中的原生引擎子进程与 Dev Server 进程树、关闭 Web 静态服与调试 WS 宿主机，防止产生僵尸进程。
- **游戏进程自行退出同步**：桌面引擎窗口被用户直接关闭时，`DebugSession` 后台监听捕获进程退出并推送 `DebugEvent::SessionStopped`，GUI 自动重置「开发」与「调试」两页的运行按钮与选中状态（通过会话代际号防旧事件串扰），同时保留 Dev Server 运行以维持热更新。
- 切换工程时自动停止当前运行中的 `DebugSession` 与 Dev Server 并退出配置编辑态，防止跨工程状态污染。
