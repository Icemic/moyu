/**
 * Engine debug-run command.
 *
 * Launches the downloaded engine in native mode (child process) or
 * web mode (local HTTP dev server with layered static file serving).
 */

import { spawn } from 'node:child_process';
import { existsSync, statSync } from 'node:fs';
import { readFile, readdir } from 'node:fs/promises';
import { createServer } from 'node:http';
import { extname, join, normalize, resolve } from 'node:path';
import { defineCommand } from 'citty';
import consola from 'consola';
import { loadMeta } from '../utils/engine.js';
import { metaFile, nativeDir, requireProjectRoot, webDir } from '../utils/project.js';

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
    const nativePath = nativeDir(projectRoot);
    const webPath = webDir(projectRoot);

    // Ensure engine is downloaded
    const meta = await loadMeta(metaPath);
    if (!meta) {
      consola.error('Engine is not downloaded yet. Run "moyu update" first.');
      process.exit(1);
    }

    const mode = args.web ? 'web' : 'native';
    const port = Number.parseInt(args.port, 10);
    if (Number.isNaN(port) || port < 1 || port > 65535) {
      consola.error('Invalid port number.');
      process.exit(1);
    }

    if (mode === 'native') {
      runNative(projectRoot, nativePath);
    } else {
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
        'The engine files may be corrupted. Run "moyu update" to re-download.',
    );
    process.exit(1);
  }

  consola.info(`Starting native engine: ${enginePath}`);
  consola.info(`Working directory: ${projectRoot}`);

  const child = spawn(enginePath, ['--entry', 'http://localhost:6020/index.json'], {
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
    consola.error('Web engine assets not found. Run "moyu update" first.');
    process.exit(1);
  }

  const entries = await readdir(webPath);
  if (entries.length === 0) {
    consola.error('Web engine directory is empty. Run "moyu update" to re-download.');
    process.exit(1);
  }

  // eslint-disable-next-line @typescript-eslint/no-misused-promises
  const server = createServer(async (req, res) => {
    const urlPath = req.url ?? '/';
    const filePath = resolveFilePath(urlPath, webPath, projectRoot);

    if (!filePath) {
      res.writeHead(404, { 'Content-Type': 'text/plain' });
      res.end('404 Not Found');
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
