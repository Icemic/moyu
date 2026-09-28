/**
 * Engine debug-run command.
 *
 * Launches the downloaded engine in native mode (child process) or
 * web mode (local HTTP dev server with layered static file serving).
 *
 * The web server also proxies to the bundler dev server (port 6020) for files that
 * only exist in the bundler's memory, so one port can serve the whole game.
 */

import { spawn } from 'node:child_process';
import { existsSync, statSync } from 'node:fs';
import { readFile, readdir } from 'node:fs/promises';
import { createServer, request as httpRequest } from 'node:http';
import type { IncomingMessage, ServerResponse } from 'node:http';
import { connect as connectTcp } from 'node:net';
import { extname, join, normalize, resolve } from 'node:path';
import type { Duplex } from 'node:stream';
import { defineCommand } from 'citty';
import consola from 'consola';
import { detectPlatform, loadMeta } from '../utils/engine.js';
import { metaFile, platformDir, requireProjectRoot } from '../utils/project.js';

/**
 * Port of the rspack dev server that framework projects run with "yarn dev".
 * Proxied to by the web server below and used as the entry for native runs.
 */
const DEV_SERVER_PORT = 6020;

export default defineCommand({
  meta: {
    name: 'run',
    description: 'Run the engine in native or web mode',
  },
  args: {
    native: {
      type: 'boolean',
      alias: 'n',
      description: 'Run in native mode (default)',
    },
    web: {
      type: 'boolean',
      alias: 'w',
      description: 'Run in web mode with a local dev server',
    },
    port: {
      type: 'string',
      description: 'Port for the web dev server',
      default: '6320',
    },
  },
  run: async ({ args }) => {
    const projectRoot = requireProjectRoot();
    const metaPath = metaFile(projectRoot);

    // Ensure engine is downloaded and active
    const meta = await loadMeta(metaPath);
    if (!meta?.active) {
      consola.error('No active engine version. Run "moyu download" first.');
      process.exit(1);
    }

    const activeVersion = meta.active.version;

    const mode = args.web ? 'web' : 'native';
    const port = Number.parseInt(args.port, 10);
    if (Number.isNaN(port) || port < 1 || port > 65535) {
      consola.error('Invalid port number.');
      process.exit(1);
    }

    if (mode === 'native') {
      const currentPlatform = detectPlatform();
      const nativePath = platformDir(projectRoot, activeVersion, currentPlatform);
      runNative(projectRoot, nativePath);
    } else {
      const webPath = platformDir(projectRoot, activeVersion, 'web-universal');
      await runWeb(projectRoot, webPath, port);
    }
  },
});

// ---------------------------------------------------------------------------
// Native mode
// ---------------------------------------------------------------------------

function runNative(projectRoot: string, nativePath: string): void {
  const exeName = process.platform === 'win32' ? 'moyu.exe' : 'moyu';
  const enginePath = join(nativePath, exeName);

  if (!existsSync(enginePath)) {
    consola.error(
      `Engine binary not found at ${enginePath}\n` +
        'The engine files may be corrupted. Run "moyu download" to re-download.',
    );
    process.exit(1);
  }

  consola.info(`Starting native engine: ${enginePath}`);
  consola.info(`Working directory: ${projectRoot}`);

  const child = spawn(enginePath, ['--entry', `http://localhost:${DEV_SERVER_PORT}/index.json`], {
    cwd: projectRoot,
    stdio: 'inherit',
  });

  child.on('error', (err) => {
    consola.error(`Failed to start engine: ${err.message}`);
    process.exit(1);
  });

  child.on('exit', (code, signal) => {
    if (signal) {
      consola.info(`Engine terminated by signal: ${signal}`);
      process.exit(1);
    }
    process.exit(code ?? 0);
  });

  // Forward termination signals to child process
  const forwardSignal = (sig: NodeJS.Signals) => {
    child.kill(sig);
  };
  process.on('SIGINT', () => forwardSignal('SIGINT'));
  process.on('SIGTERM', () => forwardSignal('SIGTERM'));
}

// ---------------------------------------------------------------------------
// Web mode – layered static file server
// ---------------------------------------------------------------------------

const MIME_TYPES: Record<string, string> = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'application/javascript; charset=utf-8',
  '.mjs': 'application/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.wasm': 'application/wasm',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.gif': 'image/gif',
  '.svg': 'image/svg+xml',
  '.ico': 'image/x-icon',
  '.webp': 'image/webp',
  '.mp3': 'audio/mpeg',
  '.ogg': 'audio/ogg',
  '.opus': 'audio/opus',
  '.wav': 'audio/wav',
  '.mp4': 'video/mp4',
  '.webm': 'video/webm',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
  '.ttf': 'font/ttf',
  '.otf': 'font/otf',
  '.txt': 'text/plain; charset=utf-8',
  '.xml': 'application/xml',
};

/**
 * Resolve a URL path to a real file path, checking the layered roots
 * in priority order: webDir first, then projectRoot.
 * Returns `null` if no matching file is found.
 */
function resolveFilePath(urlPath: string, webPath: string, projectRoot: string): string | null {
  const clean = decodeURIComponent(urlPath.split('?')[0].split('#')[0]);
  const relative = normalize(clean).replace(/^[\\/]+/, '');

  const roots = [webPath, projectRoot];

  for (const root of roots) {
    const candidate = resolve(root, relative);

    // Security: prevent path traversal outside the root
    if (!candidate.startsWith(root)) continue;

    if (!existsSync(candidate)) continue;

    const st = statSync(candidate);
    if (st.isFile()) return candidate;

    // Directory -> try index.html
    if (st.isDirectory()) {
      const index = join(candidate, 'index.html');
      if (existsSync(index) && statSync(index).isFile()) return index;
    }
  }

  return null;
}

async function runWeb(projectRoot: string, webPath: string, port: number): Promise<void> {
  if (!existsSync(webPath)) {
    consola.error('Web engine assets not found. Run "moyu download" to download web-universal platform.');
    process.exit(1);
  }

  const entries = await readdir(webPath);
  if (entries.length === 0) {
    consola.error('Web engine directory is empty. Run "moyu download" to re-download.');
    process.exit(1);
  }

  // eslint-disable-next-line @typescript-eslint/no-misused-promises
  const server = createServer(async (req, res) => {
    const urlPath = req.url ?? '/';
    const filePath = resolveFilePath(urlPath, webPath, projectRoot);

    if (!filePath) {
      // Bundler output (chunks, HMR manifests) is not on disk; ask the dev server.
      proxyToDevServer(req, res);
      return;
    }

    const ext = extname(filePath).toLowerCase();
    const contentType = MIME_TYPES[ext] ?? 'application/octet-stream';

    const headers: Record<string, string> = {
      'Content-Type': contentType,
      'Access-Control-Allow-Origin': '*',
    };

    // Required for SharedArrayBuffer (used by WASM threads)
    if (ext === '.html') {
      headers['Cross-Origin-Embedder-Policy'] = 'require-corp';
      headers['Cross-Origin-Opener-Policy'] = 'same-origin';
    }

    try {
      const data = await readFile(filePath);
      res.writeHead(200, headers);
      res.end(data);
    } catch {
      res.writeHead(500, { 'Content-Type': 'text/plain' });
      res.end('500 Internal Server Error');
    }
  });

  // The HMR client connects to the port the page was served from, so upgrades
  // have to be forwarded to the dev server as well.
  server.on('upgrade', (req, socket, head) => {
    const pathname = (req.url ?? '').split('?')[0];
    if (pathname !== '/ws') {
      socket.destroy();
      return;
    }
    proxyUpgrade(req, socket, head);
  });

  // Try to bind to the requested port; auto-increment on EADDRINUSE
  const maxRetries = 10;
  let currentPort = port;

  const tryListen = (): Promise<void> =>
    new Promise((resolve, reject) => {
      server.once('error', (err: NodeJS.ErrnoException) => {
        if (err.code === 'EADDRINUSE' && currentPort < port + maxRetries) {
          currentPort++;
          consola.warn(`Port ${currentPort - 1} in use, trying ${currentPort}...`);
          tryListen().then(resolve, reject);
        } else {
          reject(err);
        }
      });
      server.listen(currentPort, () => resolve());
    });

  await tryListen();

  const url = `http://localhost:${currentPort}`;
  consola.success(`Web engine server running at ${url}`);
  consola.info('Serving files from:');
  consola.info(`  1. ${webPath} (engine)`);
  consola.info(`  2. ${projectRoot} (project)`);
  consola.info('Press Ctrl+C to stop.\n');

  // Graceful shutdown
  process.on('SIGINT', () => {
    consola.info('\nShutting down server...');
    server.close(() => process.exit(0));
  });
  process.on('SIGTERM', () => {
    server.close(() => process.exit(0));
  });

  // Keep the process alive
  await new Promise(() => {});
}

// ---------------------------------------------------------------------------
// Dev server proxying
// ---------------------------------------------------------------------------

/** Set after the first unreachable-dev-server warning to avoid repeating it. */
let devServerWarned = false;

/**
 * Forward a request to the rspack dev server. Only reached when the layered file
 * lookup misses, which covers files that live in the bundler's memory only
 * (bundle chunks, HMR manifests).
 */
function proxyToDevServer(req: IncomingMessage, res: ServerResponse): void {
  const proxyReq = httpRequest(
    {
      host: '127.0.0.1',
      port: DEV_SERVER_PORT,
      method: req.method,
      path: req.url,
      headers: { ...req.headers, host: `127.0.0.1:${DEV_SERVER_PORT}` },
    },
    (proxyRes) => {
      res.writeHead(proxyRes.statusCode ?? 502, proxyRes.headers);
      proxyRes.pipe(res);
    },
  );

  proxyReq.on('error', () => {
    if (!devServerWarned) {
      devServerWarned = true;
      consola.warn(
        `Dev server not reachable on port ${DEV_SERVER_PORT}; bundler output will fail to load.`,
      );
    }

    if (!res.headersSent) {
      res.writeHead(502, { 'Content-Type': 'text/plain; charset=utf-8' });
    }
    res.end(`502 Bad Gateway: no dev server on port ${DEV_SERVER_PORT}. Start it and reload.`);
  });

  req.pipe(proxyReq);
}

/**
 * Forward a WebSocket upgrade (HMR) to the rspack dev server. Node has no
 * built-in upgrade proxy, so the request line is rewritten by hand and both
 * directions are piped.
 */
function proxyUpgrade(req: IncomingMessage, socket: Duplex, head: Buffer): void {
  const target = connectTcp({ host: '127.0.0.1', port: DEV_SERVER_PORT }, () => {
    const headerLines = Object.entries({ ...req.headers, host: `127.0.0.1:${DEV_SERVER_PORT}` })
      .map(([name, value]) => `${name}: ${Array.isArray(value) ? value.join(', ') : value}`)
      .join('\r\n');

    target.write(`${req.method ?? 'GET'} ${req.url ?? '/'} HTTP/1.1\r\n${headerLines}\r\n\r\n`);
    if (head.length > 0) {
      target.write(head);
    }

    socket.pipe(target);
    target.pipe(socket);
  });

  target.on('error', () => socket.destroy());
  socket.on('error', () => target.destroy());
}
