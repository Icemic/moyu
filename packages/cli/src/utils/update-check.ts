/**
 * CLI version update checker.
 *
 * Checks the npm registry for newer versions of @momoyu-ink/cli at most
 * once per day. Results are cached in `~/.moyu/update-check.json`.
 * All network errors are silently swallowed so this never blocks the user.
 */

import { existsSync, readFileSync } from 'node:fs';
import { mkdir, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { dirname, join } from 'node:path';
import consola from 'consola';
import pc from 'picocolors';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

declare const __CLI_VERSION__: string;

const PACKAGE_NAME = '@momoyu-ink/cli';
const CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000; // 24 hours
const CACHE_DIR = join(homedir(), '.moyu');
const CACHE_FILE = join(CACHE_DIR, 'update-check.json');
const REGISTRY_URL = `https://registry.npmjs.org/${PACKAGE_NAME}/latest`;

interface CacheData {
  lastChecked: string;
  latestVersion: string;
}

// ---------------------------------------------------------------------------
// Version comparison (simple semver: major.minor.patch)
// ---------------------------------------------------------------------------

function parseVersion(v: string): number[] {
  return v.replace(/^v/, '').split('.').map(Number);
}

function isNewer(latest: string, current: string): boolean {
  const a = parseVersion(latest);
  const b = parseVersion(current);
  for (let i = 0; i < 3; i++) {
    if ((a[i] ?? 0) > (b[i] ?? 0)) return true;
    if ((a[i] ?? 0) < (b[i] ?? 0)) return false;
  }
  return false;
}

// ---------------------------------------------------------------------------
// Cache I/O
// ---------------------------------------------------------------------------

function readCache(): CacheData | null {
  if (!existsSync(CACHE_FILE)) return null;
  try {
    return JSON.parse(readFileSync(CACHE_FILE, 'utf-8'));
  } catch {
    return null;
  }
}

async function writeCache(data: CacheData): Promise<void> {
  try {
    await mkdir(dirname(CACHE_FILE), { recursive: true });
    await writeFile(CACHE_FILE, JSON.stringify(data, null, 2) + '\n');
  } catch {
    // Non-critical – ignore write failures
  }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Get the current CLI version injected at build time.
 */
export function getCurrentVersion(): string {
  return typeof __CLI_VERSION__ !== 'undefined' ? __CLI_VERSION__ : '0.0.0';
}

/**
 * Check if a newer version of the CLI is available.
 *
 * Returns the latest version string if an update is available,
 * or `null` if the CLI is up to date (or the check is skipped / fails).
 */
export async function checkForUpdates(): Promise<string | null> {
  try {
    const currentVersion = getCurrentVersion();
    const cache = readCache();

    // Use cached result if still fresh
    if (cache) {
      const age = Date.now() - new Date(cache.lastChecked).getTime();
      if (age < CHECK_INTERVAL_MS) {
        return isNewer(cache.latestVersion, currentVersion) ? cache.latestVersion : null;
      }
    }

    // Fetch latest version from npm registry
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 5000);

    const res = await fetch(REGISTRY_URL, {
      signal: controller.signal,
      headers: { Accept: 'application/json' },
    });
    clearTimeout(timeout);

    if (!res.ok) return null;

    const data = await res.json();
    const latestVersion: string = data.version;

    // Update cache
    await writeCache({
      lastChecked: new Date().toISOString(),
      latestVersion,
    });

    return isNewer(latestVersion, currentVersion) ? latestVersion : null;
  } catch {
    // Network errors, timeouts, parse errors – all silent
    return null;
  }
}

/**
 * Print a boxed update notification if a new version is available.
 */
export function printUpdateNotice(latestVersion: string): void {
  const currentVersion = getCurrentVersion();
  const message = [
    '',
    pc.yellow('  Update available: ') + pc.dim(currentVersion) + pc.yellow(' -> ') + pc.green(latestVersion),
    pc.dim('  Run ') + pc.cyan(`npm i -g ${PACKAGE_NAME}`) + pc.dim(' to update'),
    '',
  ].join('\n');

  consola.box(message);
}
