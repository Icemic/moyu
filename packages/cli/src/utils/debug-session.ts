/**
 * One debug session: a WebSocket server the engine connects back to, plus the engine
 * process (or browser page) that connects to it.
 *
 * Shared by `moyu debug` and `moyu mcp`, which drive the same session through
 * different front ends. The protocol itself is described in
 * `rfcs/2026-09-25-runtime-debug-bridge.md`.
 */

import { spawn } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { once } from 'node:events';
import { existsSync } from 'node:fs';
import type { AddressInfo } from 'node:net';
import { join } from 'node:path';
import { WebSocketServer, type WebSocket } from 'ws';
import { detectPlatform, loadMeta } from './engine.js';
import { log } from './log.js';
import { metaFile, platformDir, requireProjectRoot } from './project.js';
import { DEV_SERVER_PORT, requireWebEngineAssets, startStaticFileServer } from './static-server.js';

/** Entry used for native debugging; the project's own dev server serves it. */
const DEFAULT_NATIVE_ENTRY = `http://localhost:${DEV_SERVER_PORT}/index.json`;
const DEFAULT_WEB_PORT = 6320;
/** How long to wait for the engine to connect unless the caller says otherwise. */
export const DEFAULT_CONNECT_TIMEOUT_MS = 60_000;
/** How long to wait for the engine to finish starting up unless the caller says otherwise. */
export const DEFAULT_READY_TIMEOUT_MS = 60_000;

/** A message exchanged with the engine; only `type` is always present. */
export type DebugMessage = { type?: string; requestId?: number; [key: string]: any };

/** How a command reaches the engine. */
export interface DebugHost {
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

/** A running session. */
export interface DebugSession {
  host: DebugHost;
  /** How the engine was started; a `web` session needs a browser to be opened. */
  mode: 'native' | 'web' | 'attach';
  /** Page to open for a `web` session. */
  url?: string;
  /** Stop the engine and the listener. Safe to call more than once. */
  stop(): void;
}

export interface SessionOptions {
  /** Defaults to the project the process is running in. */
  projectRoot?: string;
  /** Debug a page instead of a native window. */
  web?: boolean;
  /** Port for the web engine server; ignored by native sessions. */
  port?: number;
  /** Entry file for a native engine. */
  entry?: string;
  /**
   * Wait for an engine someone else started, listening on this fixed port.
   *
   * Nothing is launched, so this is how a packaged build gets inspected: the engine
   * is told where to connect through `MOYU_ENGINE_DEBUG_WS` and connects on its own.
   */
  attachPort?: number;
}

/**
 * Start a debug session: listen for an engine, then launch one that connects back.
 *
 * Native sessions return as soon as the engine process is spawned, web sessions as
 * soon as the page is served, attach sessions as soon as the port is bound; none of
 * them waits for the engine to connect, so a caller reports progress on its own terms.
 */
export async function startDebugSession(options: SessionOptions = {}): Promise<DebugSession> {
  if (options.attachPort !== undefined) {
    return startAttachSession(options.attachPort);
  }

  const projectRoot = options.projectRoot ?? requireProjectRoot();
  const meta = await loadMeta(metaFile(projectRoot));

  if (!meta?.active) {
    throw new Error('No active engine version. Run "moyu download" first.');
  }

  const sessionId = randomUUID();
  const host = await startHost(sessionId);
  const endpoint = `ws://127.0.0.1:${host.port}/debug/ws?sessionId=${sessionId}&role=engine`;

  log.info(`Debug endpoint listening on port ${host.port}`);

  try {
    const engine = options.web
      ? await launchWebEngine(
          projectRoot,
          meta.active.version,
          endpoint,
          sessionId,
          options.port ?? DEFAULT_WEB_PORT,
        )
      : launchNativeEngine(projectRoot, meta.active.version, endpoint, sessionId, options.entry);

    return {
      host,
      mode: options.web ? 'web' : 'native',
      url: engine.url,
      stop: () => {
        engine.stop();
        host.close();
      },
    };
  } catch (error) {
    host.close();
    throw error;
  }
}

/**
 * Listen for an engine that was started by hand, such as a packaged build.
 *
 * The engine only connects when it is told where to, so the endpoint is printed as the
 * environment variable its startup reads.
 */
async function startAttachSession(port: number): Promise<DebugSession> {
  const sessionId = randomUUID();
  const host = await startHost(sessionId, port);
  const endpoint = `ws://127.0.0.1:${host.port}/debug/ws?sessionId=${sessionId}&role=engine`;

  log.info(`Waiting for an engine on port ${host.port}. Start it with:`);
  log.info(`  MOYU_ENGINE_DEBUG_WS=${endpoint}`);

  return {
    host,
    mode: 'attach',
    stop: () => host.close(),
  };
}

// ---------------------------------------------------------------------------
// Debug host
// ---------------------------------------------------------------------------

/** Listen on `port`, or let the OS pick a free one when it is 0. */
async function startHost(sessionId: string, requestedPort = 0): Promise<DebugHost> {
  const server = new WebSocketServer({ host: '127.0.0.1', port: requestedPort });
  await once(server, 'listening');

  const port = (server.address() as AddressInfo).port;
  const pending = new Map<number, { resolve(message: DebugMessage): void; reject(error: Error): void }>();
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
        log.info(`Engine connected: ${message.platform} ${message.engineVersion} (entry ${message.entry})`);
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
    log.warn(`Ignoring malformed message: ${text}`);
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
  url?: string;
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
    throw new Error(
      `Engine binary not found at ${enginePath}\n` +
        'The engine files may be corrupted. Run "moyu download" to re-download.',
    );
  }

  const params = JSON.stringify({ engineDebugWsUrl: endpoint, engineDebugSessionId: sessionId });

  log.info(`Starting native engine: ${enginePath}`);

  // The engine logs to its own stdout, which is forwarded to our stderr: our stdout is
  // reserved for command results, and for the MCP server it carries the protocol.
  const child = spawn(enginePath, ['--entry', entry ?? DEFAULT_NATIVE_ENTRY, '--params', params], {
    cwd: projectRoot,
    stdio: ['ignore', process.stderr, 'inherit'],
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

  // The engine reads both the entry and the debug endpoint from the page query, so a
  // browser that opens this URL starts a session without any further setup.
  const query = new URLSearchParams({
    entry: `${server.url}/index.json`,
    engineDebugWsUrl: endpoint,
    engineDebugSessionId: sessionId,
  });

  const url = `${server.url}/?${query}`;
  log.info(`Web engine server running at ${server.url}`);
  log.info(`Open this page to start the engine: ${url}`);

  return {
    url,
    stop: () => server.close(),
  };
}
