# RFC：Moyu JSON Schema 扩展约定

- **状态**：提议中
- **日期**：2026-07-28
- **作者**：末语项目组
- **适用范围**：Moyu framework 导出的 `commands.schema.json` 与 `ui.schema.json`
- **相关实现**：`packages/kit/src/zod-patch.ts`、framework 的 `src/commands/commands.ts` 与 `src/data/ui.ts`

## 摘要

本文整理 Moyu 在标准 JSON Schema 之外增加的字段和 `format` 取值，作为 framework 与编辑器交换 Schema 时的统一约定。

当前扩展包括：

- 4 个 `x-*` 字段：`x-asset-kind`、`x-step`、`x-i18n`、`x-i18n-desc`；
- 7 个自定义 `format` 取值：`asset`、`character`、`color`、`point`、`bounds`、`position`、`text-style`。

这些扩展用于补充编辑信息，不代替 `type`、`minimum`、`maximum`、`multipleOf` 等标准 JSON Schema 验证规则。

## `x-*` 扩展字段

### `x-asset-kind`

标记资源字段所引用的资源类型。

```ts
type AssetKind = 'audio' | 'image' | 'font' | 'video' | 'other';
```

约定：

- 仅用于 `format: "asset"` 的字段；
- `format: "asset"` 时应同时提供；
- 不检查路径、扩展名或资源是否存在；
- 当前 framework 实际使用 `audio`、`image` 和 `video`，`font` 与 `other` 为已定义但尚未使用的类型。

示例：

```json
{
  "type": "string",
  "format": "asset",
  "x-asset-kind": "image"
}
```

### `x-step`

指定数字编辑时建议采用的变化步长。

```ts
type NumberEditorMeta = {
  'x-step'?: number;
};
```

`x-step` 只影响编辑操作，不限制数据必须是该值的整数倍。需要验证步长时仍应使用标准字段 `multipleOf`。

```json
{
  "type": "number",
  "minimum": 0,
  "multipleOf": 1,
  "x-step": 50
}
```

上例要求值为整数，编辑时建议每次变化 50。当前产物中使用的值有 `0.01`、`0.1`、`1` 和 `50`。

### `x-i18n`

提供 `title` 的多语言文案。

```ts
type SchemaI18n = Record<string, string>;
```

Key 为 locale，value 为对应语言的标题。没有匹配文案时使用标准字段 `title`。

```json
{
  "title": "Background",
  "x-i18n": {
    "zh-CN": "背景"
  }
}
```

该字段可用于命令、页面、对象、字段和联合分支。

### `x-i18n-desc`

提供 `description` 的多语言文案，结构与 `x-i18n` 相同。没有匹配文案时使用标准字段 `description`。

```json
{
  "description": "Background image asset",
  "x-i18n-desc": {
    "zh-CN": "背景图片资源"
  }
}
```

## 自定义 `format`

`format` 是标准 JSON Schema 字段，但以下取值由 Moyu 自行定义。

| `format` | 数据结构 | 使用位置 | 含义 |
| --- | --- | --- | --- |
| `asset` | string | commands、UI | 项目资源引用 |
| `character` | string | commands | 角色引用 |
| `color` | string | commands、UI | 颜色值 |
| `point` | 两元素 number tuple | commands、UI | 二维点 |
| `bounds` | 四元素 number tuple | UI | 九宫格边界 |
| `position` | `{ x: number, y: number }` | UI | 二维坐标对象 |
| `text-style` | object | UI | 文字样式配置 |

### `asset`

标记项目资源引用。字段应同时提供 `x-asset-kind`。

```json
{
  "type": "string",
  "format": "asset",
  "x-asset-kind": "audio"
}
```

### `character`

标记角色引用。

```json
{
  "type": "string",
  "format": "character"
}
```

### `color`

标记颜色字符串。`format: "color"` 本身不验证颜色语法；需要限制格式时应另加 `pattern` 等标准规则。

```json
{
  "type": "string",
  "format": "color"
}
```

### `point`

标记由两个数字组成的二维点。坐标单位和范围由字段说明及标准数值约束决定。

```json
{
  "type": "array",
  "prefixItems": [
    { "type": "number" },
    { "type": "number" }
  ],
  "minItems": 2,
  "maxItems": 2,
  "format": "point"
}
```

### `bounds`

标记由四个数字组成的九宫格边界，顺序为 left、top、right、bottom。

```json
{
  "type": "array",
  "prefixItems": [
    { "type": "number" },
    { "type": "number" },
    { "type": "number" },
    { "type": "number" }
  ],
  "minItems": 4,
  "maxItems": 4,
  "format": "bounds"
}
```

### `position`

标记包含 `x`、`y` 两个数字字段的坐标对象。坐标单位和范围由子字段定义。

```json
{
  "type": "object",
  "properties": {
    "x": { "type": "number" },
    "y": { "type": "number" }
  },
  "required": ["x", "y"],
  "format": "position"
}
```

### `text-style`

标记文字样式对象。对象内可包含字号、颜色、行高、缩进、描边和阴影等字段，各字段仍使用标准 JSON Schema 规则描述类型与限制。

```json
{
  "type": "object",
  "format": "text-style",
  "properties": {
    "fontSize": { "type": "number", "minimum": 0 },
    "fillColor": { "type": "string", "format": "color" }
  }
}
```

## 类型声明

Kit 当前通过 Zod `GlobalMeta` 声明这些扩展：

```ts
declare module 'zod' {
  interface GlobalMeta {
    title?: string;
    format?: string;
    'x-asset-kind'?: 'audio' | 'image' | 'font' | 'video' | 'other';
    'x-step'?: number;
    'x-i18n'?: Record<string, string>;
    'x-i18n-desc'?: Record<string, string>;
  }
}
```

Framework 在 Zod `.meta()` 中填写这些字段，生成后保留在 `commands.schema.json` 或 `ui.schema.json` 中。

## 扩展规则

新增或修改扩展时，应同步更新：

1. Kit 的 Zod metadata 类型；
2. Framework 中的 Schema 定义与生成产物；
3. 本文档；
4. 使用该扩展的编辑器。

未知扩展不能承担数据验证职责。需要验证的数据约束应使用标准 JSON Schema 字段表达。
