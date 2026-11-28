# @momoyu-ink/cli

[末语](https://momoyu.ink)视觉小说引擎开发工具链（`moyu-devtool`）的 npm 包装入口包。

本包通过 `optionalDependencies` 按平台安装预编译的 `moyu-devtool` 原生二进制（`@momoyu-ink/cli-<platform>`），并将 `moyu <command>` 参数与标准输入输出直接透传给 `moyu-devtool`。完整命令参考、调试桥用法与 MCP 工具说明见 [crates/devtool/README.md](../../crates/devtool/README.md)。

## 使用

不必安装，直接在项目目录里运行：

```bash
npx @momoyu-ink/cli <command>
```

也可以装为项目开发依赖，可执行文件名是 `moyu`：

```bash
npm install --save-dev @momoyu-ink/cli
npx moyu <command>
```

查看全部命令与选项：

```bash
npx moyu --help
```

## 环境变量

- `MOYU_DEVTOOL_BIN`：覆盖自动解析的 `moyu-devtool` 二进制路径（用于本地调试或自定义构建）。

## 许可

MPL-2.0
