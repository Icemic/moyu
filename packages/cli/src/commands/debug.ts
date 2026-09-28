/**
 * Debug bridge host.
 *
 * Starts a WebSocket server that engines connect to, launches an engine with the
 * debug endpoint injected, and runs one of the debug requests against it. The
 * protocol is described in `rfcs/2026-09-25-runtime-debug-bridge.md`.
 */

import { spawn } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { once } from 'node:events';
import { existsSync } from 'node:fs';
import type { AddressInfo } from 'node:net';
import { join } from 'node:path';
import { defineCommand } from 'citty';
import consola from 'consola';
import { WebSocketServer, type WebSocket } from 'ws';
import { detectPlatform, loadMeta } from '../utils/engine.js';
import { metaFile, platformDir, requireProjectRoot } from '../utils/project.js';
import {
  DEV_SERVER_PORT,
  requireWebEngineAssets,
  startStaticFileServer,
} from '../utils/static-server.js';

/** Entry used for native debugging; the project's own dev server serves it. */
const DEFAULT_NATIVE_ENTRY = `http://localhost:${DEV_SERVER_PORT}/index.json`;
const DEFAULT_WEB_PORT = 6320;
const DEFAULT_TIMEOUT_MS = 60_000;
/** Levels of children `tree` prints unless asked for another number. */
const DEFAULT_TREE_DEPTH = 8;

/** Options shared by every subcommand: how to reach or start the engine. */
const sessionArgs = {
  web: {
    type: 'boolean',
    alias: 'w',
    description: 'Debug an engine running in a browser page',
  },
  port: {
    type: 'string',
    description: 'Port for the web engine server',
    default: String(DEFAULT_WEB_PORT),
  },
  entry: {
    type: 'string',
    description: 'Entry file passed to a native engine',
  },
  timeout: {
    type: 'string',
    description: 'Milliseconds to wait for the engine to connect',
    default: String(DEFAULT_TIMEOUT_MS),
  },
} as const;

export default defineCommand({
  meta: {
    name: 'debug',
    description: 'Inspect a running engine through the debug bridge',
  },
  subCommands: {
    state: defineCommand({
      meta: {
        name: 'state',
        description: 'Print the state snapshot of an engine',
      },
      args: sessionArgs,
      run: ({ args }) =>
        runSession(args, async (host) => {
          await host.waitForReady(Number(args.timeout));
          consola.log(JSON.stringify(await host.request('engine:state'), null, 2));
        }),
    }),
    eval: defineCommand({
      meta: {
        name: 'eval',
        description: 'Evaluate JavaScript in the engine and print the result',
      },
      args: {
        ...sessionArgs,
        code: {
          type: 'positional',
          description: 'JavaScript to evaluate',
          required: true,
        },
        'eval-timeout': {
          type: 'string',
          description: 'Milliseconds the engine may spend on the evaluation',
          default: '5000',
        },
      },
      run: ({ args }) =>
        runSession(args, async (host) => {
          // Evaluating before the project script has run would report nothing useful.
          await host.waitForReady(Number(args.timeout));

          const result = await host.request('engine:eval', {
            code: args.code,
            timeoutMs: Number(args['eval-timeout']),
          });

          consola.log(
            result.value === undefined ? result.repr : JSON.stringify(result.value, null, 2),
          );

          if (result.truncated === true) {
            consola.warn('Result exceeded the size limit and is only shown in short form.');
          }
        }),
    }),
    logs: defineCommand({
      meta: {
        name: 'logs',
        description: 'Read engine logs, optionally following new entries',
      },
      args: {
        ...sessionArgs,
        level: {
          type: 'string',
          description: 'Only entries at this level or more severe',
        },
        limit: {
          type: 'string',
          description: 'Maximum number of entries to return',
        },
        since: {
          type: 'string',
          description: 'Only entries newer than this sequence number',
        },
        follow: {
          type: 'boolean',
          alias: 'f',
          description: 'Keep printing new entries until interrupted',
        },
      },
      run: ({ args }) =>
        runSession(args, async (host) => {
          // Pushes may arrive before the response that carries the snapshot, and some of
          // them are also part of that snapshot, so they wait here until it is printed.
          const pending: LogEntry[] = [];
          let watching = false;
          let lastSeq = 0;

          host.onMessage((message) => {
            if (message.type !== 'engine:log') return;

            if (watching) {
              printNewEntry(message.entry);
            } else {
              pending.push(message.entry);
            }
          });

          const result = await host.request('engine:logs', {
            level: args.level,
            limit: args.limit === undefined ? undefined : Number(args.limit),
            sinceSeq: args.since === undefined ? undefined : Number(args.since),
            subscribe: Boolean(args.follow),
          });

          const printNewEntry = (entry: LogEntry) => {
            // Entries at or below the cursor are either already printed or part of the
            // snapshot; a jump means the engine dropped pushes for a slow client.
            if (entry.seq <= lastSeq) return;

            lastSeq = entry.seq;
            printLogEntry(entry);
          };

          for (const entry of result.entries) {
            printLogEntry(entry);
            lastSeq = Math.max(lastSeq, entry.seq);
          }

          if (!args.follow) {
            return;
          }

          watching = true;

          for (const entry of pending) {
            printNewEntry(entry);
          }

          pending.length = 0;
          if (result.dropped > 0) {
            consola.warn(`${result.dropped} earlier entries were dropped.`);
          }

          consola.info('Following. Press Ctrl+C to stop.');

          // The session stays open for as long as the user watches the logs.
          await waitForInterrupt();
        }),
    }),
    tree: defineCommand({
      meta: {
        name: 'tree',
        description: 'Print the node tree',
      },
      args: {
        ...sessionArgs,
        node: {
          type: 'positional',
          description: 'Node id to start from (defaults to the root)',
          required: false,
        },
        depth: {
          type: 'string',
          description: 'Levels of children to print',
          default: String(DEFAULT_TREE_DEPTH),
        },
      },
      run: ({ args }) =>
        runSession(args, async (host) => {
          await host.waitForReady(Number(args.timeout));

          const result = await host.request('engine:tree', {
            nodeId: args.node === undefined ? undefined : Number(args.node),
            depth: Number(args.depth),
          });

          printNode(result.node, 0);
        }),
    }),
    props: defineCommand({
      meta: {
        name: 'props',
        description: 'Print one node properties and derived state',
      },
      args: {
        ...sessionArgs,
        node: {
          type: 'positional',
          description: 'Node id to read',
          required: true,
        },
      },
      run: ({ args }) =>
        runSession(args, async (host) => {
          await host.waitForReady(Number(args.timeout));

          const result = await host.request('engine:props', { nodeId: Number(args.node) });
          consola.log(JSON.stringify(result.node, null, 2));
        }),
    }),
  },
});

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

/** One node of a tree response. */
interface NodeSummary {
  id: number;
  type: string;
  label: string;
  visible: boolean;
  children: NodeSummary[];
}

/** Print a node and its children as an indented list. */
function printNode(node: NodeSummary, indent: number) {
  const label = node.label === '' ? '' : ` ${JSON.stringify(node.label)}`;
  const hidden = node.visible ? '' : ' (hidden)';

  consola.log(`${'  '.repeat(indent)}#${node.id} ${node.type}${label}${hidden}`);

  for (const child of node.children) {
    printNode(child, indent + 1);
  }
}

type DebugMessage = { type?: string; requestId?: number; [key: string]: any };

/** One entry of the engine's log buffer, as sent by `engine:logs` and `engine:log`. */
interface LogEntry {
  seq: number;
  level: string;
  target: string;
  message: string;
  timestampMs: number;
}

interface SessionOptions {
  web?: boolean;
  port: string;
  entry?: string;
  timeout: string;
}

/**
 * Start the host and an engine, run `task` against the connected engine, then shut
 * everything down. A task that needs the session to stay open simply does not return
 * until it is done watching.
 */
async function runSession(options: SessionOptions, task: (host: DebugHost) => Promise<void>) {
  const projectRoot = requireProjectRoot();
  const meta = await loadMeta(metaFile(projectRoot));

  if (!meta?.active) {
    consola.error('No active engine version. Run "moyu download" first.');
    process.exit(1);
  }

  const sessionId = randomUUID();
  const host = await startHost(sessionId);
  const endpoint = `ws://127.0.0.1:${host.port}/debug/ws?sessionId=${sessionId}&role=engine`;

  consola.info(`Debug endpoint listening on port ${host.port}`);

  const engine = options.web
    ? await launchWebEngine(
        projectRoot,
        meta.active.version,
        endpoint,
        sessionId,
        Number(options.port),
      )
    : launchNativeEngine(projectRoot, meta.active.version, endpoint, sessionId, options.entry);

  let exitCode = 0;

  try {
    await host.waitForEngine(Number(options.timeout));
    await task(host);
  } catch (error) {
    consola.error(error instanceof Error ? error.message : String(error));
    exitCode = 1;
  } finally {
    engine.stop();
    host.close();
  }

  process.exit(exitCode);
}

/** Resolve when the user interrupts the process. */
function waitForInterrupt(): Promise<void> {
  return new Promise((resolve) => {
    process.once('SIGINT', () => resolve());
    process.once('SIGTERM', () => resolve());
  });
}

function printLogEntry(entry: DebugMessage) {
  const stamp = new Date(entry.timestampMs).toISOString().slice(11, 23);
  consola.log(`${stamp} ${String(entry.level).toUpperCase()} ${entry.target}: ${entry.message}`);
}

// ---------------------------------------------------------------------------
// Debug host
// ---------------------------------------------------------------------------

interface DebugHost {
  /** Port the endpoint listens on; the OS picks a free one. */
  port: number;
  /** Wait until an engine connects. */
  waitForEngine(timeoutMs: number): Promise<void>;
  /** Wait until the engine has finished starting up. */
  waitForReady(timeoutMs: number): Promise<void>;
  /** Send a request and wait for its `:done` or `:error` answer. */
  request(type: string, payload?: Record<string, unknown>): Promise<DebugMessage>;
  /** Observe every message the engine sends, including pushes. */
  onMessage(handler: (message: DebugMessage) => void): void;
  close(): void;
}

async function startHost(sessionId: string): Promise<DebugHost> {
  const server = new WebSocketServer({ host: '127.0.0.1', port: 0 });
  await once(server, 'listening');

  const port = (server.address() as AddressInfo).port;
  const pending = new Map<
    number,
    { resolve(message: DebugMessage): void; reject(error: Error): void }
  >();
  const observers: ((message: DebugMessage) => void)[] = [];

  let nextRequestId = 1;
  let socket: WebSocket | null = null;
  let engineConnected = () => {};
  let engineFailed = (_error: Error) => {};

  const connected = new Promise<void>((resolve, reject) => {
    engineConnected = resolve;
    engineFailed = reject;
  });

  // The engine reports whether it finished starting up in `engine:hello`, and pushes
  // `engine:ready` when that happens after the connection was already open.
  let engineReady = false;
  let readyWaiters: (() => void)[] = [];

  const markReady = () => {
    engineReady = true;

    for (const waiter of readyWaiters) {
      waiter();
    }

    readyWaiters = [];
  };

  server.on('connection', (connection) => {
    socket = connection;

    connection.on('message', (data) => {
      const message = parseMessage(String(data));
      if (!message) return;

      if (message.type === 'engine:hello') {
        consola.info(
          `Engine connected: ${message.platform} ${message.engineVersion} (entry ${message.entry})`,
        );
        engineConnected();

        if (message.ready === true) {
          markReady();
        }
        return;
      }

      if (message.type === 'engine:ready') {
        markReady();
        return;
      }
      for (const observer of observers) {
        observer(message);
      }

      const requestId = message.requestId;
      if (typeof requestId !== 'number') return;

      const handler = pending.get(requestId);
      if (!handler) return;

      pending.delete(requestId);

      if (typeof message.type === 'string' && message.type.endsWith(':error')) {
        const details = message.stack ? `\n${String(message.stack)}` : '';
        handler.reject(new Error(`${String(message.message)}${details}`));
      } else {
        handler.resolve(message);
      }
    });

    connection.on('close', () => {
      socket = null;

      for (const handler of pending.values()) {
        handler.reject(new Error('Engine disconnected before answering.'));
      }
      pending.clear();
    });
  });

  server.on('error', (error: Error) => engineFailed(error));

  return {
    port,
    waitForEngine: (timeoutMs) =>
      withTimeout(
        connected,
        timeoutMs,
        `No engine connected within ${timeoutMs}ms.\n` +
          'Check that the engine is running, and that its version supports the debug bridge.',
      ),
    waitForReady: (timeoutMs) => {
      if (engineReady) {
        return Promise.resolve();
      }

      return withTimeout(
        new Promise<void>((resolve) => readyWaiters.push(resolve)),
        timeoutMs,
        `The engine did not finish starting up within ${timeoutMs}ms.`,
      );
    },
    request: (type, payload) =>
      new Promise((resolve, reject) => {
        if (!socket) {
          reject(new Error('No engine is connected.'));
          return;
        }

        const requestId = nextRequestId++;
        pending.set(requestId, { resolve, reject });

        socket.send(JSON.stringify({ type, requestId, sessionId, ...payload }));
      }),
    onMessage: (handler) => {
      observers.push(handler);
    },
    close: () => server.close(),
  };
}

function parseMessage(text: string): DebugMessage | null {
  try {
    const message: unknown = JSON.parse(text);
    return typeof message === 'object' && message !== null ? (message as DebugMessage) : null;
  } catch {
    consola.warn(`Ignoring malformed message: ${text}`);
    return null;
  }
}

function withTimeout<T>(promise: Promise<T>, ms: number, message: string): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(message)), ms);

    promise.then(
      (value) => {
        clearTimeout(timer);
        resolve(value);
      },
      (error: unknown) => {
        clearTimeout(timer);
        reject(error instanceof Error ? error : new Error(String(error)));
      },
    );
  });
}

// ---------------------------------------------------------------------------
// Engine launch
// ---------------------------------------------------------------------------

interface LaunchedEngine {
  stop(): void;
}

function launchNativeEngine(
  projectRoot: string,
  version: string,
  endpoint: string,
  sessionId: string,
  entry: string | undefined,
): LaunchedEngine {
  const exeName = process.platform === 'win32' ? 'moyu.exe' : 'moyu';
  const enginePath = join(platformDir(projectRoot, version, detectPlatform()), exeName);

  if (!existsSync(enginePath)) {
    consola.error(
      `Engine binary not found at ${enginePath}\n` +
        'The engine files may be corrupted. Run "moyu download" to re-download.',
    );
    process.exit(1);
  }

  const params = JSON.stringify({ engineDebugWsUrl: endpoint, engineDebugSessionId: sessionId });

  consola.info(`Starting native engine: ${enginePath}`);

  const child = spawn(enginePath, ['--entry', entry ?? DEFAULT_NATIVE_ENTRY, '--params', params], {
    cwd: projectRoot,
    stdio: 'inherit',
  });

  child.on('error', (err) => {
    consola.error(`Failed to start engine: ${err.message}`);
    process.exit(1);
  });

  return {
    stop: () => child.kill(),
  };
}

async function launchWebEngine(
  projectRoot: string,
  version: string,
  endpoint: string,
  sessionId: string,
  port: number,
): Promise<LaunchedEngine> {
  const webPath = await requireWebEngineAssets(projectRoot, version);
  const server = await startStaticFileServer({ projectRoot, webPath, port });

  // The engine reads the debug endpoint from the page query; the entry page itself
  // resolves its own entry, so no entry override is needed here.
  const query = new URLSearchParams({
    engineDebugWsUrl: endpoint,
    engineDebugSessionId: sessionId,
  });

  consola.success(`Web engine server running at ${server.url}`);
  consola.info(`Open this page to start the engine:\n  ${server.url}/?${query}\n`);

  return {
    stop: () => server.close(),
  };
}
