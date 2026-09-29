/**
 * Model Context Protocol over stdio.
 *
 * Frames are newline-delimited JSON-RPC 2.0 on stdin and stdout. Nothing else may write
 * to stdout: a stray line would be read as a frame and break the connection, which is
 * why every diagnostic in this package goes to stderr.
 *
 * Only the parts a tool server needs are implemented: the handshake, tool listing, and
 * tool calls.
 */

import { log } from '../utils/log.js';

/**
 * Versions this server answers in. The handshake echoes whatever the client asks for,
 * because the tool surface is the same across them; when the client asks about
 * discovery instead, these are the versions advertised.
 */
const SUPPORTED_VERSIONS = ['2026-07-28', '2025-06-18', '2025-03-26', '2024-11-05'];
const NEWEST_VERSION = SUPPORTED_VERSIONS[0];

/** What a tool call returns: content blocks the host renders, plus an error flag. */
export type ToolContent =
  | { type: 'text'; text: string }
  | { type: 'image'; data: string; mimeType: string };

export interface ToolResult {
  content: ToolContent[];
  isError?: boolean;
}

export interface ToolDefinition {
  name: string;
  title: string;
  description: string;
  /** JSON Schema for the arguments, as the host validates and the model reads them. */
  inputSchema: Record<string, unknown>;
  run(args: Record<string, unknown>): Promise<ToolResult>;
}

export interface ServerInfo {
  name: string;
  version: string;
}

interface Request {
  jsonrpc: '2.0';
  id?: string | number | null;
  method: string;
  params?: Record<string, unknown>;
}

/**
 * Serve MCP on stdio until stdin closes.
 *
 * `tools` is read on every listing, so a caller may change it at runtime.
 */
export function serveMcp(options: {
  info: ServerInfo;
  tools: () => ToolDefinition[];
  /** Called before the process exits, however that happens. */
  onShutdown: () => void;
}): void {
  const tools = new Map(options.tools().map((tool) => [tool.name, tool]));

  let buffer = '';
  let shuttingDown = false;
  /** Requests still running; the server outlives its input until they answer. */
  let pending = 0;
  /** Writes not yet handed to the stream; leaving early would truncate them. */
  let unflushed = 0;
  let inputEnded = false;

  const shutdown = () => {
    if (shuttingDown) return;
    shuttingDown = true;

    options.onShutdown();
    process.exit(0);
  };

  /**
   * Leave once the input is gone, no answer is still owed, and everything written has
   * reached the stream. Writes to a pipe complete asynchronously, so exiting right after
   * writing would drop the last replies.
   */
  const shutdownWhenIdle = () => {
    if (inputEnded && pending === 0 && unflushed === 0) {
      shutdown();
    }
  };

  const send = (message: unknown) => {
    unflushed++;

    process.stdout.write(`${JSON.stringify(message)}\n`, () => {
      unflushed--;
      shutdownWhenIdle();
    });
  };

  const sendResult = (id: Request['id'], result: unknown) => {
    send({ jsonrpc: '2.0', id, result });
  };

  const sendError = (id: Request['id'], code: number, message: string) => {
    send({ jsonrpc: '2.0', id, error: { code, message } });
  };

  const handle = async (request: Request) => {
    // Notifications carry no id and expect no answer.
    const isNotification = request.id === undefined || request.id === null;

    switch (request.method) {
      case 'initialize':
        sendResult(request.id, {
          protocolVersion: typeof request.params?.protocolVersion === 'string' ? request.params.protocolVersion : NEWEST_VERSION,
          capabilities: { tools: {} },
          serverInfo: options.info,
        });
        return;

      case 'server/discover':
        sendResult(request.id, {
          resultType: 'complete',
          supportedVersions: SUPPORTED_VERSIONS,
          capabilities: { tools: {} },
          _meta: { 'io.modelcontextprotocol/serverInfo': options.info },
        });
        return;

      case 'tools/list':
        sendResult(request.id, { tools: [...tools.values()].map(describeTool) });
        return;

      case 'tools/call':
        await callTool(request, sendResult, sendError);
        return;

      case 'ping':
        sendResult(request.id, {});
        return;

      case 'notifications/initialized':
      case 'notifications/cancelled':
        return;

      default:
        // Answering with an error is friendlier than silence: the client knows the
        // method was seen and rejected, rather than waiting for a reply that never comes.
        if (!isNotification) {
          sendError(request.id, -32601, `Method not found: ${request.method}`);
        }
    }
  };

  const callTool = async (
    request: Request,
    reply: (id: Request['id'], result: unknown) => void,
    fail: (id: Request['id'], code: number, message: string) => void,
  ) => {
    const name = request.params?.name;
    const tool = typeof name === 'string' ? tools.get(name) : undefined;

    if (!tool) {
      fail(request.id, -32602, `Unknown tool: ${String(name)}`);
      return;
    }

    const args = (request.params?.arguments ?? {}) as Record<string, unknown>;

    try {
      reply(request.id, await tool.run(args));
    } catch (error) {
      // A failing tool is a normal result the model should see and react to, so it is
      // reported as content rather than as a protocol error.
      const message = error instanceof Error ? error.message : String(error);

      reply(request.id, { content: [{ type: 'text', text: message }], isError: true });
    }
  };

  process.stdin.setEncoding('utf8');
  process.stdin.on('data', (chunk: string) => {
    buffer += chunk;

    let newline = buffer.indexOf('\n');
    while (newline !== -1) {
      const line = buffer.slice(0, newline).trim();
      buffer = buffer.slice(newline + 1);

      if (line !== '') {
        dispatch(line);
      }

      newline = buffer.indexOf('\n');
    }
  });

  // The host closes stdin when it is done with the server. Answers already being
  // prepared are still delivered, so a host that pipes its requests and closes the
  // stream still receives every reply.
  process.stdin.on('end', () => {
    inputEnded = true;
    shutdownWhenIdle();
  });
  process.on('SIGINT', shutdown);
  process.on('SIGTERM', shutdown);

  function dispatch(line: string) {
    let request: Request;

    try {
      request = JSON.parse(line) as Request;
    } catch {
      log.warn(`Ignoring unreadable message: ${line}`);
      return;
    }

    pending++;

    // Each request is handled on its own so that a slow tool cannot hold up the stream.
    void handle(request)
      .catch((error: unknown) => {
        log.error(`Request ${request.method} failed: ${error instanceof Error ? error.message : String(error)}`);

        if (request.id !== undefined && request.id !== null) {
          sendError(request.id, -32603, 'Internal error');
        }
      })
      .finally(() => {
        pending--;
        shutdownWhenIdle();
      });
  }
}

function describeTool(tool: ToolDefinition) {
  return {
    name: tool.name,
    title: tool.title,
    description: tool.description,
    inputSchema: tool.inputSchema,
  };
}
