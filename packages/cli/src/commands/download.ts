/**
 * Engine download command.
 *
 * Interactive wizard that lets the user pick a channel, version, and one or
 * more platforms to download. Supports cross-platform downloads (e.g.
 * downloading Linux engine on Windows for pack).
 */

import { defineCommand } from 'citty';
import consola from 'consola';
import type { DownloadedVersion } from '../utils/engine.js';
import {
  tryDetectPlatform,
  downloadAndExtract,
  fetchVersionsJson,
  loadConfig,
  loadMeta,
  saveMeta,
} from '../utils/engine.js';
import { metaFile, platformDir, requireProjectRoot } from '../utils/project.js';

export default defineCommand({
  meta: {
    name: 'download',
    description: 'Download a specific engine version for selected platforms',
  },
  run: async () => {
    const projectRoot = requireProjectRoot();
    const metaPath = metaFile(projectRoot);

    // 1. Read config & fetch manifest
    const config = loadConfig(projectRoot);
    consola.start('Fetching version manifest...');
    const versions = await fetchVersionsJson(config.cdnUrl);

    // 2. Select channel
    const channelNames = Object.keys(versions.channels);
    if (channelNames.length === 0) {
      throw new Error('No channels available in the version manifest.');
    }

    const selectedChannel = await consola.prompt('Select channel:', {
      type: 'select',
      cancel: 'symbol',
      options: channelNames.map((name) => ({
        label: name,
        value: name,
        hint: name === config.channel ? 'current' : undefined,
      })),
    });
    if (typeof selectedChannel === 'symbol') {
      consola.info('Download cancelled.');
      return;
    }

    const channel = versions.channels[selectedChannel];

    // 3. Select version
    const versionNames = Object.keys(channel.versions);
    if (versionNames.length === 0) {
      throw new Error(`No versions available in channel "${selectedChannel}".`);
    }

    const selectedVersion = await consola.prompt('Select version:', {
      type: 'select',
      cancel: 'symbol',
      options: versionNames.map((name) => ({
        label: name,
        value: name,
        hint: name === channel.latest ? 'latest' : undefined,
      })),
    });
    if (typeof selectedVersion === 'symbol') {
      consola.info('Download cancelled.');
      return;
    }

    const entry = channel.versions[selectedVersion];

    // 4. Select platforms (multiselect)
    const availablePlatforms = Object.keys(entry.assets);
    if (availablePlatforms.length === 0) {
      throw new Error(`No platform assets available for version "${selectedVersion}".`);
    }

    const currentPlatform = tryDetectPlatform();

    const selectedPlatforms = await consola.prompt('Select platforms to download:', {
      type: 'multiselect',
      cancel: 'symbol',
      options: availablePlatforms,
      initial: currentPlatform && availablePlatforms.includes(currentPlatform) ? [currentPlatform] : undefined,
    });
    if (typeof selectedPlatforms === 'symbol') {
      consola.info('Download cancelled.');
      return;
    }

    const platforms = selectedPlatforms;
    if (platforms.length === 0) {
      consola.warn('No platforms selected. Nothing to download.');
      return;
    }

    // 5. Load metadata
    const meta = (await loadMeta(metaPath)) ?? { active: null, downloads: {} };

    // 6. Check which platforms actually need downloading
    const existingDownload = meta.downloads[selectedVersion];
    const toDownload: string[] = [];
    for (const p of platforms) {
      const local = existingDownload?.platforms[p];
      const remote = entry.assets[p];
      if (local && local.sha256 === remote.sha256) {
        consola.info(`${p}: already up to date, skipping.`);
      } else {
        toDownload.push(p);
      }
    }

    if (toDownload.length === 0) {
      consola.success('All selected platforms are already downloaded.');
    } else {
      // 7. Download & extract in parallel
      const tasks: Promise<void>[] = [];
      for (const p of toDownload) {
        const asset = entry.assets[p];
        const destDir = platformDir(projectRoot, selectedVersion, p);
        consola.start(`Downloading ${p}...`);
        tasks.push(downloadAndExtract(asset.url, destDir, asset.sha256));
      }
      await Promise.all(tasks);
      consola.success(`Downloaded ${toDownload.length} platform(s) for ${selectedVersion}.`);
    }

    // 8. Update metadata
    const now = new Date().toISOString();
    const platformRecords: DownloadedVersion['platforms'] = {};
    for (const p of platforms) {
      platformRecords[p] = {
        sha256: entry.assets[p].sha256,
        downloadedAt: existingDownload?.platforms[p]?.downloadedAt ?? now,
      };
    }
    // Newly downloaded platforms get fresh timestamp
    for (const p of toDownload) {
      platformRecords[p] = {
        sha256: entry.assets[p].sha256,
        downloadedAt: now,
      };
    }

    meta.downloads[selectedVersion] = {
      channel: selectedChannel,
      publishedAt: entry.published_at,
      platforms: {
        ...(existingDownload?.platforms ?? {}),
        ...platformRecords,
      },
    };

    // 9. Ask whether to switch active version
    if (!meta.active) {
      // No active version yet – auto-activate
      meta.active = { version: selectedVersion, channel: selectedChannel };
      consola.info(`Active version set to ${selectedVersion} (first download).`);
    } else if (meta.active.version !== selectedVersion) {
      const switchActive = await consola.prompt(
        `Switch active version from ${meta.active.version} to ${selectedVersion}?`,
        { type: 'confirm', initial: false, cancel: 'symbol' },
      );
      if (switchActive && typeof switchActive !== 'symbol') {
        meta.active = { version: selectedVersion, channel: selectedChannel };
        consola.info(`Active version switched to ${selectedVersion}.`);
      }
    }

    await saveMeta(metaPath, meta);
    consola.success('Done!');
  },
});
