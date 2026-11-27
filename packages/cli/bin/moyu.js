#!/usr/bin/env node

import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const PLATFORM_PACKAGES = {
  'win32-x64': { pkg: '@momoyu-ink/cli-win32-x64', bin: 'moyu-devtool.exe' },
  'linux-x64': { pkg: '@momoyu-ink/cli-linux-x64', bin: 'moyu-devtool' },
  'linux-arm64': { pkg: '@momoyu-ink/cli-linux-arm64', bin: 'moyu-devtool' },
  'darwin-x64': { pkg: '@momoyu-ink/cli-darwin-x64', bin: 'moyu-devtool' },
  'darwin-arm64': { pkg: '@momoyu-ink/cli-darwin-arm64', bin: 'moyu-devtool' },
};

function resolveBinary() {
  const override = process.env.MOYU_DEVTOOL_BIN;
  if (override) {
    if (!existsSync(override)) {
      throw new Error(`MOYU_DEVTOOL_BIN points to a missing file: ${override}`);
    }
    return override;
  }

  const key = `${process.platform}-${process.arch}`;
  const entry = PLATFORM_PACKAGES[key];
  if (!entry) {
    const supported = Object.keys(PLATFORM_PACKAGES).join(', ');
    throw new Error(`Unsupported platform: ${key}. Supported platforms: ${supported}`);
  }

  const require = createRequire(import.meta.url);
  try {
    return require.resolve(`${entry.pkg}/bin/${entry.bin}`);
  } catch {
    // Fall back to workspace Cargo build outputs when running inside the monorepo.
    const binDir = dirname(fileURLToPath(import.meta.url));
    const repoRoot = resolve(binDir, '..', '..', '..');
    for (const profile of ['release', 'debug']) {
      const localBin = resolve(repoRoot, 'target', profile, entry.bin);
      if (existsSync(localBin)) {
        return localBin;
      }
    }

    throw new Error(
      `Could not find prebuilt binary package "${entry.pkg}".\n` +
        'Make sure optional dependencies are enabled when installing @momoyu-ink/cli, ' +
        'or build the workspace binary with `cargo build -p moyu_devtool`.',
    );
  }
}

let binaryPath;
try {
  binaryPath = resolveBinary();
} catch (err) {
  process.stderr.write(`error: ${err instanceof Error ? err.message : String(err)}\n`);
  process.exit(1);
}

const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
});

child.on('error', (err) => {
  process.stderr.write(`error: Failed to start ${binaryPath}: ${err.message}\n`);
  process.exit(1);
});

const forwardSignal = (signal) => {
  if (!child.killed) {
    child.kill(signal);
  }
};
process.on('SIGINT', () => forwardSignal('SIGINT'));
process.on('SIGTERM', () => forwardSignal('SIGTERM'));

child.on('exit', (code, signal) => {
  if (signal) {
    process.exit(1);
  }
  process.exit(code ?? 0);
});
