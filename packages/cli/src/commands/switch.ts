/**
 * Engine version switch command.
 *
 * Lists all locally downloaded engine versions and lets the user
 * pick which one to set as the active version.
 */

import { defineCommand } from 'citty';
import consola from 'consola';
import { detectPlatform, loadMeta, saveMeta } from '../utils/engine.js';
import { metaFile, requireProjectRoot } from '../utils/project.js';

export default defineCommand({
  meta: {
    name: 'switch',
    description: 'Switch the active engine version',
  },
  run: async () => {
    const projectRoot = requireProjectRoot();
    const metaPath = metaFile(projectRoot);

    // 1. Load metadata
    const meta = await loadMeta(metaPath);
    if (!meta || Object.keys(meta.downloads).length === 0) {
      consola.error('No engine versions downloaded yet. Run "moyu download" first.');
      process.exit(1);
    }

    const versionNames = Object.keys(meta.downloads);

    // 2. If only one version, nothing to switch
    if (versionNames.length === 1) {
      const only = versionNames[0];
      if (meta.active?.version === only) {
        consola.info(`Only one version available (${only}) and it is already active.`);
      } else {
        meta.active = { version: only, channel: meta.downloads[only].channel };
        await saveMeta(metaPath, meta);
        consola.success(`Active version set to ${only}.`);
      }
      return;
    }

    // 3. Interactive selection
    const selected = await consola.prompt('Select engine version to activate:', {
      type: 'select',
      cancel: 'symbol',
      options: versionNames.map((v) => {
        const dl = meta.downloads[v];
        const platforms = Object.keys(dl.platforms).join(', ');
        return {
          label: v,
          value: v,
          hint: `${dl.channel} | ${platforms}${meta.active?.version === v ? ' (active)' : ''}`,
        };
      }),
    });
    if (typeof selected === 'symbol') {
      consola.info('Switch cancelled.');
      return;
    }

    if (meta.active?.version === selected) {
      consola.info(`${selected} is already the active version.`);
      return;
    }

    // 4. Warn if the selected version doesn't include the current platform
    const currentPlatform = detectPlatform();
    const dl = meta.downloads[selected];
    if (!dl.platforms[currentPlatform]) {
      consola.warn(
        `Version ${selected} does not include platform "${currentPlatform}".\n` +
          'Native run may not work. Use "moyu download" to add it.',
      );
    }

    // 5. Update active
    meta.active = { version: selected, channel: meta.downloads[selected].channel };
    await saveMeta(metaPath, meta);

    consola.success(`Active version switched to ${selected}.`);
  },
});
