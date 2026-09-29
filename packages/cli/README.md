# @momoyu-ink/cli

[末语](https://momoyu.ink)视觉小说引擎的命令行工具：项目脚手架、引擎下载与切换、运行项目、打包、Schema 生成，以及运行时调试。

## 使用

不必安装，直接在项目目录里运行：

```bash
npx @momoyu-ink/cli <command>
```

也可以装为项目依赖，可执行文件名是 `moyu`：

```bash
npm install --save-dev @momoyu-ink/cli
npx moyu <command>
```

每个命令都有自己的帮助，例如 `moyu debug --help`。

## 命令

| 命令 | 用途 |
| --- | --- |
| `moyu init` | 创建新项目。交互流程已经就绪，脚手架本身尚未实现。 |
| `moyu download` | 下载指定引擎版本与平台。 |
| `moyu update` | 在当前 channel 内升级到最新引擎版本。 |
| `moyu switch` | 在已下载的引擎版本之间切换当前版本。 |
| `moyu run` | 运行引擎：native 模式（`--native`）或为浏览器提供本地服务（`--web`）。 |
| `moyu debug` | 通过调试桥查看运行中的引擎。 |
| `moyu mcp` | 把调试桥作为 [MCP](https://modelcontextprotocol.io) 工具提供在 stdio 上。 |
| `moyu pack` | 打包游戏用于分发，可输出为 zip。 |
| `moyu schema` | 从项目的 Zod 命令定义生成 `commands.schema.json`。 |
| `moyu ui-schema` | 从项目的 Zod UI 定义生成 `ui.schema.json`。 |

需要项目的命令会在当前目录及其上层查找 `index.json` 来确定项目根。下载的引擎存放在项目的 `.moyu/engine/` 下。

## 调试运行中的引擎

`moyu debug` 与引擎之间通过 WebSocket 通信，协议见 `rfcs/2026-09-25-runtime-debug-bridge.md`。每个子命令都是一次完整会话：先监听引擎端点，再启动引擎，问一个问题，然后收尾。

```bash
moyu debug state                              # entry、平台、surface 尺寸、节点数、已运行时长
moyu debug eval "globalThis.__gameState"      # 在引擎里执行 JavaScript
moyu debug logs --level warn --limit 50       # 读取引擎日志缓冲
moyu debug logs --follow                      # 持续打印新日志
moyu debug tree --depth 8                     # 打印节点树
moyu debug props 12                           # 单个节点：JS 设置的属性与引擎算出的值
moyu debug screenshot shot.webp --max-width 1280 --max-height 720
```

`state` / `eval` / `tree` / `props` / `screenshot` 会等引擎启动完成再发请求，因此读到的是已经跑完项目脚本的状态；引擎还在初始化时，这类命令会一直等到它完成。

要检查手动启动的引擎（例如打包好的成品），加上 `--attach`。这时不会启动任何东西，命令只监听一个端口，并打印引擎需要设置的环境变量：

```bash
moyu debug state --attach --listen 6321
# info: Waiting for an engine on port 6321. Start it with:
# info:   MOYU_ENGINE_DEBUG_WS=ws://127.0.0.1:6321/debug/ws?sessionId=...&role=engine
```

命令结果写 stdout，可直接管道；进度与错误写 stderr：

```bash
moyu debug tree | grep sprite
```

## 从 AI 宿主调试

`moyu mcp` 把同样的能力作为 [Model Context Protocol](https://modelcontextprotocol.io) 工具提供，AI 宿主可以直接启动引擎并查看它。它在 stdio 上说 JSON-RPC，stdout 只承载协议帧。

```jsonc
// .vscode/mcp.json
{
  "servers": {
    "moyu": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "@momoyu-ink/cli", "mcp"]
    }
  }
}
```

| 工具 | 用途 |
| --- | --- |
| `debug_start` | 带着调试桥启动引擎并等它就绪；会先停掉上一个会话。 |
| `debug_stop` | 停止 `debug_start` 启动的引擎与监听。 |
| `debug_state` | 引擎状态快照。 |
| `debug_eval` | 在引擎里执行 JavaScript。 |
| `debug_logs` | 读取日志缓冲，可用序号续读。 |
| `debug_tree` | 列出节点树。 |
| `debug_props` | 单个节点的属性与派生值。 |
| `debug_screenshot` | 截图，以图像形式返回。 |

会话在工具调用之间保持存活，因此可以启动一次引擎、反复查询。默认的 native 会话会打开窗口；传 `web: true` 改为提供项目页面，并返回一个需要在浏览器里打开的地址。

## 许可

MPL-2.0
