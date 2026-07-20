# RFC：末语布局系统

- **状态**：已接受
- **日期**：2026-07-20
- **作者**：末语项目组
- **适用范围**：`moyu`、`@momoyu-ink/kit`
- **相关实现**：`crates/core/src/nodes/node.rs`、`crates/core/src/nodes/container.rs`、`crates/nodes/src/nodes/linear_layout.rs`

## 摘要

本文定义末语节点布局系统的长期语义。布局系统包含两种并存的布局方式：

1. **传统布局（legacy layout）**：节点通过 `x/y`、`anchor`、`pivot` 和视觉变换在父节点中绝对定位。
2. **单轴布局（flex layout）**：`VBox` 和 `HBox` 按顺序测量并排列直接子节点，提供内容包裹、显式尺寸、间距、内边距和常用对齐能力。

两种方式共享同一套尺寸、测量、排列和变换基础。传统布局继续负责舞台定位、覆盖关系和自由变换；单轴布局用于菜单、表单、按钮组等顺序排列结构。它们可以任意嵌套。

## 背景

末语原有节点系统采用绝对定位。节点的 `x/y` 表示相对父节点的位置，`anchor` 以父节点尺寸为基准，`pivot` 以自身尺寸为基准，缩放、旋转和斜切组成局部变换。这种方式适合视觉小说中的舞台构图、立绘、文本框和叠加效果，但顺序排列的界面需要手工计算每一项坐标。

在加入 `VBox` 和 `HBox` 前，节点尺寸主要由 renderer 在更新阶段产生，尺寸准备、变换和渲染发生在同一遍树遍历中。这会使父节点无法在当前帧使用子节点的新尺寸，深层嵌套还可能按层级逐帧收敛。

为支持可靠的自动布局，引擎需要先确定本帧尺寸，再排列节点并计算变换，最后更新渲染资源。同时，传统布局不能被自动布局替代：两者解决的问题不同，必须保持清晰且可组合的职责边界。

## 目标

本 RFC 规定以下内容：

- 传统布局的坐标、anchor、pivot 和视觉变换语义；
- 普通 `Container` 与 `Filter` 的自动测量语义；
- `VBox`、`HBox` 的公开 API、测量和排列规则；
- intrinsic size、layout size、layout position 的职责；
- 布局、视觉边界和渲染之间的生命周期；
- 两种布局嵌套时的组合规则；
- 可见性、溢出、动态尺寸和特殊 wrapper 节点的行为。

## 非目标

本 RFC 不定义完整 CSS Flexbox，也不提供以下能力：

- `flexGrow`、`flexShrink`、`flexBasis`、wrap、order；
- margin、`alignSelf`、stretch、`space-around`、`space-evenly`；
- 百分比尺寸、min/max 约束、视口单位和父约束传播；
- 单轴布局中的绝对定位子项或跳过布局属性；
- 类似 CSS `display: none` 的布局排除能力；
- 自动裁剪溢出内容；
- Grid 或其他多轴约束布局；
- JS 侧布局计算或公开查询 layout rect 的命令式 API。

## 术语

### 传统布局

本文用“传统布局”指末语原有的绝对定位模型。英文文档和代码讨论中可称为 `legacy layout` 或 `absolute layout`。

### 单轴布局

本文用“单轴布局”指 `VBox` 和 `HBox` 提供的顺序排列模型。它只实现常用 Flexbox 语义的子集。英文文档和代码讨论中可称为 `flex layout` 或 `linear layout`。

### 内在尺寸（intrinsic size）

节点内容或 renderer 产生的自然尺寸，例如图片原始区域、文本排版结果、动画帧尺寸或视频画面尺寸。

### 布局尺寸（layout size）

节点本帧用于父级测量、anchor、pivot、变换和渲染几何的最终未变换尺寸。

### 布局位置（layout position）

`VBox` 或 `HBox` 为直接子节点分配的未变换矩形左上角。布局位置与用户设置的 `x/y` 分开保存。

### 视觉边界（content bounds）

包含节点自身和子树视觉变换结果的轴对齐包围盒。它主要用于渲染可见性判断、子树视觉范围聚合和部分离屏效果，不作为自动布局占位尺寸。命中由 `Focusable` 在节点局部空间中判断，Clip 则使用自身布局矩形经过全局变换后的结果。

## 总体设计

### 两种布局并存

末语不向 `Container` 增加 `layout` 或 `flexDirection`。传统布局继续使用普通节点和 `Container`；单轴布局使用独立的 `VBox` 和 `HBox` 节点。

选择独立节点的原因：

- `Container` 继续表达分组、覆盖和绝对定位；
- JSX 能直接表达排列方向；
- 单轴布局 API 不需要承诺完整 Flexbox；
- 两种布局可以通过嵌套自然组合，不需要模式切换属性。

### 通用节点状态

`NodeBase` 保存三类互不替代的数据：

| 数据 | 生产者 | 主要用途 |
| --- | --- | --- |
| intrinsic size | 节点或 renderer prepare | 自然内容尺寸 |
| layout size | 节点 measure | 父级测量、anchor、pivot、几何 |
| layout position | `VBox` / `HBox` arrange | 单轴布局直接子项的位置 |

用户设置的 `x/y` 始终保存在 `translate` 中。布局不得改写它。

## 布局生命周期

每个图形更新帧按以下三个阶段处理逻辑节点树：

```mermaid
flowchart LR
  A[Prepare] --> B[自底向上 Measure]
  B --> C[自顶向下 Arrange 与 Transform]
  C --> D[自底向上 Visual Bounds]
  D --> E[Renderer Update 与 Command Collection]
```

### 阶段 A：准备与测量

进入节点时，renderer 的 `prepare` 处理会改变内在尺寸的工作，包括资源就绪、文本排版、动画和视频尺寸刷新。

离开节点时，子节点已经完成测量，当前节点执行 `measure`：

- 叶子节点通常把 intrinsic size 作为 layout size；
- `Container` 和 `Filter` 根据直接子节点测量；
- `VBox` 和 `HBox` 根据子项、gap、padding 和显式尺寸测量；
- 根节点不按内容测量，而是使用舞台逻辑尺寸。

### 阶段 B：排列、变换和视觉边界

进入节点时：

1. 父布局节点为直接子节点写入 layout position；
2. 节点根据父级、layout size、layout position 和用户变换计算 local/global transform；
3. 当前节点执行 `arrange`，为下一层直接子节点分配位置。

离开节点时，当前节点根据自身 layout rect 和子节点变换后的视觉边界计算 `content_bounds` 与 `global_content_bounds`。

### 阶段 C：渲染更新和命令收集

renderer 使用本帧最终的 layout size、transform 和 global content bounds 更新 GPU 资源、顶点和绘制命令。这个阶段不得首次决定本帧布局尺寸。

阶段 A、B 使用逻辑树遍历，不因 `visible=false` 或渲染 shadow 跳过节点。阶段 C 才遵守可见性和渲染 shadow 规则。因此不可见节点仍能参与测量与排列。

阶段 A、B 始终使用节点树的原始 children 顺序。阶段 C 在每个父节点下按直接子节点的 `zIndex` 稳定升序遍历：数值小的先绘制，数值大的后绘制，相同值保持原始 children 顺序。整个子树作为直接子节点的绘制单元，因此后代不能越过父节点的兄弟节点。

## 传统布局规范

### 坐标与变换

不受单轴布局父级控制的节点使用传统布局。设：

- 用户位置为 $T=(x,y)$；
- 父节点 layout size 为 $(W_p,H_p)$；
- anchor 比例为 $(a_x,a_y)$；
- 节点 layout size 为 $(W,H)$；
- pivot 比例为 $(p_x,p_y)$；
- scale、rotation、skew 组成线性变换矩阵 $M$。

则：

$$
A=(a_xW_p,a_yH_p)
$$

$$
P=(p_xW,p_yH)
$$

$$
translation=T+A-MP
$$

由此得到以下规则：

- `x/y` 是相对父节点的绝对位置偏移；
- `anchor` 是相对父节点 layout size 的比例；
- `pivot` 是相对自身 layout size 的比例；
- `pivot` 决定节点局部原点以及缩放、旋转、斜切的中心；
- scale、rotation、skew 改变视觉结果，不改变父级测量占位。

`anchor` 和 `pivot` 都是归一化比例，而不是像素坐标。常用值包括 `[0, 0]`、`[0.5, 0.5]` 和 `[1, 1]`，但引擎不要求值必须位于 $[0,1]$ 范围内。

### 普通 Container 与 Filter 的自动测量

普通 `Container` 和 `Filter` 不排列子节点，但会根据直接子节点的未变换布局矩形计算自身 layout size，使其能够作为单轴布局中的复合子项。

对直接子节点 $i$，设 authored position 为 $(x_i,y_i)$，layout size 为 $(w_i,h_i)$，pivot 比例为 $(p_{x,i},p_{y,i})$，则：

$$
right_i=x_i-p_{x,i}w_i+w_i
$$

$$
bottom_i=y_i-p_{y,i}h_i+h_i
$$

父节点尺寸为：

$$
W=\max(0,right_1,\ldots,right_n)
$$

$$
H=\max(0,bottom_1,\ldots,bottom_n)
$$

测量规则如下：

- 只读取直接子节点的 layout size；
- 计入子节点的 `x/y` 和 pivot 对未变换矩形位置的影响；
- 不计入 anchor，避免父尺寸与子 anchor 互相依赖；
- 不计入 scale、rotation、skew 后的视觉扩展；
- 负坐标内容可以向左或向上溢出，但不会移动父节点局部原点；
- 不参与父级测量的特殊 wrapper 节点会被跳过。

该测量只定义 layout size。视觉溢出仍由 content bounds 表达。

### 根节点

根 `Container` 的 layout size 每帧固定为舞台逻辑尺寸。根节点不执行普通 Container 的内容包裹测量。它为顶层 anchor 和布局提供稳定尺寸基准。

## 单轴布局规范

### 公开节点

Rust 和 `@momoyu-ink/kit` 公开两个节点：

- `<vbox>`：按纵向排列直接子节点；
- `<hbox>`：按横向排列直接子节点。

二者都是普通末语 Node，支持通用 `x/y`、anchor、pivot、scale、rotation、skew、visible、opacity 等属性，也支持 React Spring 的 `animated.vbox` 和 `animated.hbox`。

### Props

| 属性 | 类型 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `width` | `number?` | auto | 显式宽度；删除属性后恢复 auto |
| `height` | `number?` | auto | 显式高度；删除属性后恢复 auto |
| `gap` | `number` | `0` | 相邻有效子项的最小间距 |
| `padding` | `number` | `0` | 四边内边距基础值 |
| `paddingX` | `number?` | 未设置 | 覆盖左右内边距 |
| `paddingY` | `number?` | 未设置 | 覆盖上下内边距 |
| `justifyContent` | `start \| center \| end \| space-between` | `start` | 主轴对齐 |
| `alignItems` | `start \| center \| end` | `start` | 交叉轴对齐 |

尺寸和间距必须是有限非负数。非法值记录警告并使用 `0`。`width` 或 `height` 被重置时恢复 auto；`paddingX/Y` 被重置时恢复继承 `padding`。

### 自动尺寸

设参与父级测量的直接子项数量为 $n$，尺寸为 $(w_i,h_i)$，实际横向和纵向 padding 为 $P_x$、$P_y$。

VBox：

$$
W_{auto}=2P_x+\max_i(w_i)
$$

$$
H_{auto}=2P_y+\sum_i h_i+\max(0,n-1)\cdot gap
$$

HBox：

$$
W_{auto}=2P_x+\sum_i w_i+\max(0,n-1)\cdot gap
$$

$$
H_{auto}=2P_y+\max_i(h_i)
$$

空布局容器的 auto size 等于两侧 padding 之和。某个轴设置显式尺寸时，该轴使用显式值，另一个轴仍可按内容测量。显式尺寸小于内容时允许溢出，不产生负间距，也不隐式裁剪。

### 主轴排列

设子项主轴尺寸之和加基础 gap 后为：

$$
C=\sum_i mainSize_i+\max(0,n-1)\cdot gap
$$

主轴可用长度与剩余空间为：

$$
available=\max(0,containerMain-2P_{main})
$$

$$
R=\max(0,available-C)
$$

排列规则：

| `justifyContent` | 起点附加偏移 | 每个间隔附加值 |
| --- | --- | --- |
| `start` | $0$ | $0$ |
| `center` | $R/2$ | $0$ |
| `end` | $R$ | $0$ |
| `space-between` | $0$ | 当 $n\ge2$ 时为 $R/(n-1)$ |

`space-between` 保留基础 `gap`。子项少于两个时等同 `start`。auto 主轴通常没有剩余空间，因此 `center`、`end` 和 `space-between` 与 `start` 没有可见差异。

### 交叉轴排列

交叉轴可用长度为：

$$
availableCross=\max(0,containerCross-2P_{cross})
$$

当子项小于可用长度时：

- `start`：附加偏移为 $0$；
- `center`：附加偏移为 $(availableCross-crossSize)/2$；
- `end`：附加偏移为 $availableCross-crossSize$。

当子项不小于可用长度时，按 `start` 放置并向结束方向溢出。

### 直接子项的变换

单轴布局只排列直接子节点。设父布局分配位置为 $L$，用户偏移为 $T=(x,y)$，节点 pivot 像素位置为 $P$，视觉变换矩阵为 $M$，则直接子项使用：

$$
translation=L+T+P-MP
$$

由此得到以下规则：

- layout position 决定未变换矩形左上角；
- `x/y` 是布局位置之后的视觉偏移，不推动兄弟节点；
- `anchor` 对单轴布局直接子项不生效；
- pivot 是视觉变换中心，未变换时不改变分配矩形的位置；
- scale、rotation、skew 不改变布局占位；
- visible 不改变布局占位。

节点离开 `VBox` 或 `HBox` 后，父级会清除 layout position，节点恢复传统布局和 anchor 语义。

### 子项参与规则

默认情况下，所有直接子节点都参与测量、项目计数、gap 和排列，包括 `visible=false` 的节点。

`Shader` 会根据非空、`space="normal"` 的 `ShaderSlot` 内容尺寸自动测量，也可以通过 `width` / `height` 显式指定任一或两个轴。因此 Shader 可以作为 `VBox` / `HBox` 的普通直接子项，参与自动尺寸、gap 和排列。

`ShaderSlot` 是 Shader 的内部渲染通道节点。它会测量直接子内容并把结果提供给父 Shader，但自身 layout size 固定为零，并返回“不参与父级测量”。直接放入 `VBox` / `HBox` 时：

- 不计入自动尺寸、项目数、gap 或 `space-between`；
- 采用零占位；
- 引擎记录警告；
- 不应作为普通布局项使用。

`space="shader"` 或空 ShaderSlot 不贡献 Shader 的自动内容尺寸。显式 `width` / `height` 只改变父 Shader 的布局尺寸，不改变 ShaderSlot 自身的零占位语义。

### 布局顺序与绘制顺序

`VBox` 和 `HBox` 始终按原始 children 顺序测量和排列。`zIndex` 只影响同一父节点下直接子节点的绘制顺序与命中优先级，不影响：

- intrinsic size、layout size 或 content bounds 的几何结果；
- 子项计数、gap、padding、自动尺寸和对齐；
- layout position、anchor、pivot 或 transform。

命中测试使用绘制顺序的逆序，优先选择视觉上最靠前的节点。相同 `zIndex` 下，后出现的兄弟节点后绘制并优先命中。

## 两种布局的组合

### 单轴布局放在传统布局中

`VBox` 和 `HBox` 自身是普通节点。放在普通父节点下时，它们按传统布局使用 `x/y`、anchor、pivot 和视觉变换；其内部继续排列直接子项。

### 传统布局分组放在单轴布局中

普通 `Container` 可以作为 `VBox` 或 `HBox` 的直接子项。它根据内部绝对定位子节点自动测量出一个 layout size，外层单轴布局把它视为一个整体排列。Container 内部仍可自由覆盖、偏移和变换内容。

### 单轴布局互相嵌套

`VBox`、`HBox` 可以任意嵌套。内层先自底向上测量，外层读取其最终 layout size；排列再自顶向下完成，因此尺寸变化能够在同一帧传播到所有祖先布局。

## 可见性、溢出和裁剪

### 可见性

`visible=false` 不影响逻辑树测量和排列，渲染阶段才会跳过不可见节点。因此不可见节点继续占据传统 Container 的测量范围或单轴布局空间。

### 溢出

普通 `Container`、`VBox` 和 `HBox` 不隐式裁剪内容。视觉变换、负坐标、用户 `x/y` 偏移或内容大于显式尺寸都可以产生溢出。

### 裁剪

需要裁剪时应显式使用 `Clip`。Clip 的 layout size 由 `width` / `height` 决定；渲染时，裁剪矩形由该布局矩形经过节点最终 global transform 转换为舞台空间 AABB，并限制在舞台范围内。它不会因子树内容或视觉溢出自动扩大布局占位或裁剪区域。

## 动态尺寸

图片加载完成、文本重新排版、动画或视频首次获得尺寸时，renderer 在 prepare 阶段更新 intrinsic size。自底向上的 measure 会在同一帧将变化传播到 Container、Filter、VBox 和 HBox；随后 arrange、transform、bounds 和 renderer update 使用新尺寸。

本设计保证同一帧完成：

$$
resource\ ready\rightarrow intrinsic\ size\rightarrow measure\rightarrow arrange\rightarrow transform\rightarrow visual\ bounds\rightarrow render
$$

## 兼容性

### 保留的行为

- 原有节点的 `x/y`、anchor、pivot 和视觉变换 API 保持可用；
- `Container` 不自动排列子节点；
- 单轴布局只影响直接子节点；
- JSX 删除属性后，通过 Patch reset 恢复各属性默认值；
- native 与 web 使用同一 Rust 布局实现，JS 不计算布局。

### 有意的行为调整

普通 `Container` 和 `Filter` 从无内容尺寸改为根据直接子节点自动测量。这是让传统分组节点能够作为单轴布局子项的基础，也是 anchor、pivot 和复合组件尺寸更一致的长期语义。

由于 Container 尺寸不再固定为零，历史代码中依赖父级尺寸、anchor 或 pivot 的节点可能出现位置变化。此类代码应明确组件边界，或按新的尺寸语义调整 anchor/pivot 用法，不恢复旧的零尺寸行为。

## 性能

布局正确性优先于首版局部优化。当前每帧执行：

1. 逻辑树 prepare 与自底向上 measure；
2. 逻辑树 arrange、transform 与 bounds；
3. 渲染树 renderer update 与命令收集。

相比原有单遍流程，这会增加树遍历和读写锁开销，但能避免异步资源和嵌套布局逐帧收敛。性能评估至少应覆盖：

- 不使用 VBox/HBox 的现有场景；
- 10 层嵌套布局；
- 100、500、1000 个直接或嵌套子项；
- 频繁文本变化和多资源异步就绪；
- 每帧只改变 `x/y`、scale 等视觉属性的场景。

只有 profile 证明存在瓶颈后，才引入 layout dirty 子树跳过或更复杂的失效传播，避免在没有数据时增加父指针和缓存状态。

## 错误处理

- `width`、`height`、gap 和 padding 接收到非有限值或负数时，记录警告并使用 `0`；
- `ShaderSlot` 直接出现在单轴布局中时，记录警告并按零占位处理；
- 内容溢出不是错误，不记录警告。

## 测试与验收

实现应覆盖以下测试组：

1. **尺寸来源**：Sprite、Text、Animation、Video、Clip、Container、Filter、空布局容器和根节点。
2. **布局组合**：VBox/HBox、互相嵌套、Container 分组、动态增加删除和重排。
3. **对齐与溢出**：全部已支持对齐、gap/padding 优先级、显式尺寸不足和大交叉轴子项。
4. **变换**：正负 `x/y`、不同 pivot、scale、rotation、skew、布局容器自身变换及父级切换。
5. **系统回归**：content bounds、命中、Clip、Filter、Backdrop、Shader、可见性、`zIndex` 和渲染命令顺序。

Shader 与绘制顺序至少覆盖：

- normal、非空 ShaderSlot 的内容变化能在同一帧更新 Shader layout size；
- Shader 作为 VBox/HBox 直接子项正常参与自动尺寸和排列；
- ShaderSlot 直接作为布局子项时保持零占位并记录警告；
- `zIndex` 改变绘制与命中优先级，但不改变 VBox/HBox 的几何排列。

最低自动验证包括：

- `cargo build`；
- bindings 导出检查；
- `@momoyu-ink/kit` 构建；
- 使用布局能力的 framework 类型检查和构建。

## 被否决的方案

### 在 Container 上增加 Flex 属性

该方案会把绝对定位分组和自动排列混为同一种节点行为，也容易让 API 扩张成不完整的 CSS Flexbox。独立 `VBox`/`HBox` 的职责更清晰。

### 只在 JavaScript 中计算布局

JS 布局无法自然获得 Rust renderer 在当前帧准备出的文本、图片、动画和视频尺寸，也会让 native/web 之间产生重复实现。布局必须在 Rust 节点树中完成。

### 使用 content bounds 作为布局占位

content bounds 包含 scale、rotation、skew 和子树视觉溢出。使用它会让视觉动画推动兄弟节点并造成布局抖动，因此布局只能使用未变换的 layout size。

### 引入完整 Flexbox 引擎

当前需求不包括 grow、shrink、wrap、约束传播等能力。引入 Yoga、Taffy 等完整布局引擎会扩大数据模型、依赖和跨边界类型成本，暂不采用。

## 后续工作

以下事项需要独立 RFC 或迭代：

- 基于 profile 引入 layout dirty 子树优化；
- 若出现明确产品需求，再设计 margin、stretch、百分比、min/max、wrap、grow/shrink 或 Grid；
- 若需要调试工具，再设计只读 layout rect 查询 API。

## 结论

末语将传统布局与单轴布局作为两套并存、可嵌套的正式能力：

- 传统布局负责自由定位、覆盖关系和视觉变换；
- `VBox`/`HBox` 负责直接子节点的顺序排列；
- intrinsic size、layout size、layout position 和 visual bounds 各自承担独立职责；
- 三阶段生命周期保证动态尺寸和嵌套布局在同一帧稳定；
- 视觉变换不反向改变布局占位。

本文所定义的语义是后续布局功能、组件实现和兼容性判断的基准。
