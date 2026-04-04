/**
 * Engine update command.
 *
 * Updates the engine to the latest version within the current channel.
 * Re-downloads the same set of platforms that the current active version has.
 * Asks for user confirmation before downloading.
 */

import { defineCommand } from 'citty';
import consola from 'consola';
import type { DownloadedVersion } from '../utils/engine.js';
import {
  detectPlatform,
  downloadAndExtract,
  fetchVersionsJson,
  loadConfig,
  loadMeta,
  saveMeta,
} from '../utils/engine.js';
import { metaFile, platformDir, requireProjectRoot } from '../utils/project.js';

export default defineCommand({
  meta: {
    name: 'update',
    description: 'Update the engine to the latest version in the current channel',
  },
  run: async () => {
    const projectRoot = requireProjectRoot();
    const metaPath = metaFile(projectRoot);

    // 1. Load existing metadata first so update stays within the active channel
    const meta = (await loadMeta(metaPath)) ?? { active: null, downloads: {} };

    // 2. Read config
    const config = loadConfig(projectRoot);
    consola.info(`CDN: ${config.cdnUrl}`);

    const currentChannel = meta.active?.channel ?? config.channel;
    consola.info(`Channel: ${currentChannel}`);

    // 3. Fetch remote version manifest
    consola.start('Fetching version manifest...');
    const versions = await fetchVersionsJson(config.cdnUrl);

    // 4. Resolve channel
    const channel = versions.channels[currentChannel];
    if (!channel) {
      const available = Object.keys(versions.channels).join(', ');
      throw new Error(`Channel "${currentChannel}" not found. Available channels: ${available}`);
    }

    // 5. Update always means latest version in the current channel.
    const targetVersion = channel.latest;
    consola.info(`Target version: ${targetVersion}`);

    const entry = channel.versions[targetVersion];
    if (!entry) {
      const available = Object.keys(channel.versions).join(', ');
      throw new Error(
        `Version "${targetVersion}" not found in channel "${currentChannel}".\n` + `Available versions: ${available}`,
      );
    }

    // 6. Determine platforms to download:
    //    Use the same platforms as the current active version, or fall back to detectPlatform().
    let targetPlatforms: string[];
    if (meta.active) {
      const activeDownload = meta.downloads[meta.active.version];
      if (activeDownload && Object.keys(activeDownload.platforms).length > 0) {
        targetPlatforms = Object.keys(activeDownload.platforms);
      } else {
        targetPlatforms = [detectPlatform()];
      }
    } else {
      targetPlatforms = [detectPlatform()];
    }

    // 7. Check if already up to date
    const existingDownload = meta.downloads[targetVersion];
    if (existingDownload) {
      const allMatch = targetPlatforms.every((p) => {
        const local = existingDownload.platforms[p];
        const remote = entry.assets[p];
        return local && remote && local.sha256 === remote.sha256;
      });
      if (meta.active?.version === targetVersion && allMatch) {
        consola.success(`Engine is already up to date (${targetVersion}). Nothing to download.`);
        return;
      }
    }

    // 8. Validate all target platforms are available
    for (const p of targetPlatforms) {
      if (!entry.assets[p]) {
        const available = Object.keys(entry.assets).join(', ');
        throw new Error(`No asset for platform "${p}" in version ${targetVersion}.\nAvailable: ${available}`);
      }
    }

    // 9. Confirm with user
    const currentVersion = meta.active?.version ?? '(none)';
    const confirmed = await consola.prompt(
      `Update engine: ${currentVersion} → ${targetVersion}?\n  Platforms: ${targetPlatforms.join(', ')}`,
      { type: 'confirm', cancel: 'symbol' },
    );
    if (!confirmed || typeof confirmed === 'symbol') {
      consola.info('Update cancelled.');
      return;
    }

    // 10. Download & extract all platforms in parallel
    const tasks: Promise<void>[] = [];
    for (const p of targetPlatforms) {
      const asset = entry.assets[p];
      const destDir = platformDir(projectRoot, targetVersion, p);
      consola.start(`Downloading ${p}...`);
      tasks.push(downloadAndExtract(asset.url, destDir, asset.sha256));
    }
    await Promise.all(tasks);

    // 11. Update metadata
    const now = new Date().toISOString();
    const platformRecords: DownloadedVersion['platforms'] = {};
    for (const p of targetPlatforms) {
      platformRecords[p] = {
        sha256: entry.assets[p].sha256,
        downloadedAt: now,
      };
    }

    // Merge with any existing platforms for this version
    const existing = meta.downloads[targetVersion];
    meta.downloads[targetVersion] = {
      channel: currentChannel,
      publishedAt: entry.published_at,
      platforms: {
        ...(existing?.platforms ?? {}),
        ...platformRecords,
      },
    };
    meta.active = { version: targetVersion, channel: currentChannel };

    await saveMeta(metaPath, meta);

    // 12. Done
    consola.success(`Engine updated to ${targetVersion}!`);
    for (const p of targetPlatforms) {
      consola.info(`  ${p}: ${platformDir(projectRoot, targetVersion, p)}`);
    }
  },
});
