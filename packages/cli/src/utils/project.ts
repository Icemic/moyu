/**
 * Project root detection and path utilities.
 *
 * Locates the project root by searching upward for `index.json` (the Moyu
 * project manifest), then derives all engine-related paths from it.
 */

import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import consola from 'consola';

/**
 * Walk up from `startDir` (defaults to `process.cwd()`) until a directory
 * containing `index.json` is found. Returns the absolute path to that
 * directory, or `null` if the filesystem root is reached first.
 */
export function findProjectRoot(startDir?: string): string | null {
  let dir = resolve(startDir ?? process.cwd());

  for (;;) {
    if (existsSync(join(dir, 'index.json'))) {
      return dir;
    }
    const parent = dirname(dir);
    if (parent === dir) break; // filesystem root
    dir = parent;
  }

  return null;
}

/**
 * Same as `findProjectRoot` but exits the process with a helpful message
 * when no project root can be found.
 */
export function requireProjectRoot(startDir?: string): string {
  const root = findProjectRoot(startDir);
  if (!root) {
    consola.error(
      'Could not locate a Moyu project (no index.json found).\n' +
        'Make sure you are running this command inside a Moyu project directory.',
    );
    process.exit(1);
  }
  return root;
}

// ---------------------------------------------------------------------------
// Derived paths – all lazily computed from a given project root.
// ---------------------------------------------------------------------------

export function engineDir(projectRoot: string): string {
  return join(projectRoot, '.moyu', 'engine');
}

export function metaFile(projectRoot: string): string {
  return join(engineDir(projectRoot), 'meta.json');
}

export function nativeDir(projectRoot: string): string {
  return join(engineDir(projectRoot), 'native');
}

export function webDir(projectRoot: string): string {
  return join(engineDir(projectRoot), 'web');
}
