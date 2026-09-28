/**
 * Layered static file server for engine web assets.
 *
 * Used by `moyu run --web` and `moyu debug --web`: files are resolved against the
 * engine's web directory first and the project root second, so engine assets always
 * win over project files with the same path.
 *
 * Requests that miss the layered lookup are proxied to the bundler dev server, which
 * keeps the project's in-memory bundle output and HMR available on the same port as
 * the game assets.
 */

import { existsSync, statSync } from 'node:fs';
import { readFile, readdir } from 'node:fs/promises';
import type { IncomingMessage, ServerResponse } from 'node:http';
import { createServer, request as httpRequest } from 'node:http';
import { connect as connectTcp } from 'node:net';
import { extname, join, normalize, resolve } from 'node:path';
import type { Duplex } from 'node:stream';
import consola from 'consola';
import { platformDir } from './project.js';

/**
 * Port of the bundler dev server that framework projects run with "yarn dev".
 * Missing files are proxied to it, and native runs load the entry file from it.
 */
export const DEV_SERVER_PORT = 6020;

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

export interface StaticFileServer {
  /** Port the server is bound to; may differ from the requested one. */
  port: number;
  /** Base URL the server is reachable at. */
  url: string;
  close(): void;
}

/**
 * Resolve the engine's web assets, exiting when they are missing or empty.
 */
export async function requireWebEngineAssets(projectRoot: string, version: string): Promise<string> {
  const webPath = platformDir(projectRoot, version, 'web-universal');

  if (!existsSync(webPath)) {
    consola.error('Web engine assets not found. Run "moyu download" to download web-universal platform.');
    process.exit(1);
  }

  const entries = await readdir(webPath);
  if (entries.length === 0) {
    consola.error('Web engine directory is empty. Run "moyu download" to re-download.');
    process.exit(1);
  }

  return webPath;
}

/**
 * Start the layered static server on `port`, moving to the next free port when it
 * is taken (up to 10 attempts).
 */
export async function startStaticFileServer(options: {
  projectRoot: string;
  webPath: string;
  port: number;
}): Promise<StaticFileServer> {
  const { projectRoot, webPath } = options;

  const server = createServer((req, res) => {
    void (async () => {
      const filePath = resolveFilePath(req.url ?? '/', webPath, projectRoot);

      if (!filePath) {
        // Bundler output (chunks, HMR manifests) is not on disk; ask the dev server.
        proxyToDevServer(req, res);
        return;
      }

      const ext = extname(filePath).toLowerCase();
      const headers: Record<string, string> = {
        'Content-Type': MIME_TYPES[ext] ?? 'application/octet-stream',
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
    })();
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

  const port = await listen(server, options.port);

  return {
    port,
    url: `http://localhost:${port}`,
    close: () => server.close(),
  };
}

/** Bind to `port`, incrementing on EADDRINUSE. Resolves with the port in use. */
function listen(server: ReturnType<typeof createServer>, port: number, attempts = 10): Promise<number> {
  return new Promise((resolvePort, reject) => {
    let currentPort = port;

    const attempt = () => {
      server.once('error', (err: NodeJS.ErrnoException) => {
        if (err.code !== 'EADDRINUSE' || currentPort >= port + attempts) {
          reject(err);
          return;
        }

        currentPort++;
        consola.warn(`Port ${currentPort - 1} in use, trying ${currentPort}...`);
        attempt();
      });

      server.listen(currentPort, () => resolvePort(currentPort));
    };

    attempt();
  });
}

/**
 * Resolve a URL path to a real file path, checking the layered roots in priority
 * order: web directory first, then project root. Returns `null` when nothing matches.
 */
function resolveFilePath(urlPath: string, webPath: string, projectRoot: string): string | null {
  const clean = decodeURIComponent(urlPath.split('?')[0].split('#')[0]);
  const relative = normalize(clean).replace(/^[\\/]+/, '');

  for (const root of [webPath, projectRoot]) {
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

// ---------------------------------------------------------------------------
// Dev server proxying
// ---------------------------------------------------------------------------

/** Set after the first unreachable-dev-server warning to avoid repeating it. */
let devServerWarned = false;

/**
 * Forward a request to the bundler dev server. Only reached when the layered file
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
      consola.warn(`Dev server not reachable on port ${DEV_SERVER_PORT}; bundler output will fail to load.`);
    }

    if (!res.headersSent) {
      res.writeHead(502, { 'Content-Type': 'text/plain; charset=utf-8' });
    }
    res.end(`502 Bad Gateway: no dev server on port ${DEV_SERVER_PORT}. Start it and reload.`);
  });

  req.pipe(proxyReq);
}

/**
 * Forward a WebSocket upgrade (HMR) to the bundler dev server. Node has no
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
