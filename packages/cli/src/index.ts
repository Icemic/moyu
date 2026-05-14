/**
 * @momoyu-ink/cli – CLI toolchain for the Moyu Visual Novel Engine.
 *
 * Entry point that registers all sub-commands and handles global
 * concerns like version display and update checking.
 */

import { defineCommand, runMain } from 'citty';
import consola from 'consola';
import { checkForUpdates, getCurrentVersion, printUpdateNotice } from './utils/update-check.js';

const version = getCurrentVersion();

// Kick off the update check immediately (non-blocking).
// The result is consumed after the command finishes.
const updateCheckPromise = checkForUpdates();

const main = defineCommand({
  meta: {
    name: 'moyu',
    version,
    description: 'CLI toolchain for the Moyu Visual Novel Engine',
  },
  subCommands: {
    init: () => import('./commands/init.js').then((m) => m.default),
    download: () => import('./commands/download.js').then((m) => m.default),
    update: () => import('./commands/update.js').then((m) => m.default),
    switch: () => import('./commands/switch.js').then((m) => m.default),
    run: () => import('./commands/run.js').then((m) => m.default),
    pack: () => import('./commands/pack.js').then((m) => m.default),
    schema: () => import('./commands/schema.js').then((m) => m.default),
    'ui-schema': () => import('./commands/ui-schema.js').then((m) => m.default),
  },
});

function isPromptCancelledError(err: unknown): boolean {
  return err instanceof Error && err.name === 'ConsolaPromptCancelledError';
}

runMain(main)
  .then(async () => {
    // Print update notice (if any) after the command completes
    const latestVersion = await updateCheckPromise;
    if (latestVersion) {
      printUpdateNotice(latestVersion);
    }
  })
  .catch((err) => {
    if (isPromptCancelledError(err)) {
      consola.info('Cancelled.');
      process.exit(0);
    }

    consola.error(err);
    process.exit(1);
  });
