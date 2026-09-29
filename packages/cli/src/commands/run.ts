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
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { defineCommand } from 'citty';
import consola from 'consola';
import { detectPlatform, loadMeta } from '../utils/engine.js';
import { metaFile, platformDir, requireProjectRoot } from '../utils/project.js';
import { DEV_SERVER_PORT, requireWebEngineAssets, startStaticFileServer } from '../utils/static-server.js';

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
      const webPath = await requireWebEngineAssets(projectRoot, activeVersion);
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

async function runWeb(projectRoot: string, webPath: string, port: number): Promise<void> {
  const server = await startStaticFileServer({ projectRoot, webPath, port });

  consola.success(`Web engine server running at ${server.url}`);
  consola.info('Serving files from:');
  consola.info(`  1. ${webPath} (engine)`);
  consola.info(`  2. ${projectRoot} (project)`);
  consola.info('Press Ctrl+C to stop.\n');

  // Graceful shutdown
  process.on('SIGINT', () => {
    consola.info('\nShutting down server...');
    server.close();
    process.exit(0);
  });
  process.on('SIGTERM', () => {
    server.close();
    process.exit(0);
  });

  // Keep the process alive
  await new Promise(() => {});
}
