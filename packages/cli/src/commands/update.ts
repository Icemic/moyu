/**
 * Engine download & update command.
 *
 * Downloads the Moyu engine binary (native) and web release package
 * from CDN, verifies integrity, and extracts them to `.moyu/engine/`.
 */

import { defineCommand } from 'citty';
import consola from 'consola';
import type { Meta } from '../utils/engine.js';
import {
  detectPlatform,
  downloadAndExtract,
  fetchVersionsJson,
  loadConfig,
  loadMeta,
  saveMeta,
} from '../utils/engine.js';
import { metaFile, nativeDir, requireProjectRoot, webDir } from '../utils/project.js';

export default defineCommand({
  meta: {
    name: 'update',
    description: 'Download or update the Moyu engine',
  },
  run: async () => {
    const projectRoot = requireProjectRoot();
    const metaPath = metaFile(projectRoot);
    const nativePath = nativeDir(projectRoot);
    const webPath = webDir(projectRoot);

    // 1. Read config
    const config = loadConfig(projectRoot);
    consola.info(`CDN: ${config.cdnUrl}`);
    consola.info(`Channel: ${config.channel}`);

    // 2. Fetch remote version manifest
    consola.start('Fetching version manifest...');
    const versions = await fetchVersionsJson(config.cdnUrl);

    // 3. Resolve channel
    const channel = versions.channels[config.channel];
    if (!channel) {
      const available = Object.keys(versions.channels).join(', ');
      throw new Error(`Channel "${config.channel}" not found. Available channels: ${available}`);
    }

    // 4. Determine target version
    const targetVersion = config.version ?? channel.latest;
    consola.info(`Target version: ${targetVersion}`);

    const entry = channel.versions[targetVersion];
    if (!entry) {
      const available = Object.keys(channel.versions).join(', ');
      throw new Error(
        `Version "${targetVersion}" not found in channel "${config.channel}".\n` + `Available versions: ${available}`,
      );
    }

    // 5. Detect native platform
    const platform = detectPlatform();
    consola.info(`Detected platform: ${platform}`);

    const nativeAsset = entry.assets[platform];
    if (!nativeAsset) {
      const supported = Object.keys(entry.assets)
        .filter((k) => k !== 'web-universal')
        .join(', ');
      throw new Error(`No native asset for platform "${platform}".\n` + `Available platforms: ${supported}`);
    }

    const webAsset = entry.assets['web-universal'] ?? null;
    if (!webAsset) {
      consola.warn('No web-universal asset available for this version.');
    }

    // 6. Check if already up to date
    const meta = await loadMeta(metaPath);
    if (meta) {
      const nativeMatch = meta.native?.sha256 === nativeAsset.sha256;
      const webMatch = !webAsset || (meta.web && meta.web.sha256 === webAsset.sha256);

      if (meta.version === targetVersion && nativeMatch && webMatch) {
        consola.success(`Engine is already up to date (${targetVersion}). Nothing to download.`);
        return;
      }
      consola.info(`Updating engine: ${meta.version} -> ${targetVersion}`);
    }

    // 7. Download & extract (native + web in parallel when both available)
    const tasks: Promise<void>[] = [];

    consola.start(`Downloading native engine (${platform})...`);
    tasks.push(downloadAndExtract(nativeAsset.url, nativePath, nativeAsset.sha256));

    if (webAsset) {
      consola.start('Downloading web engine...');
      tasks.push(downloadAndExtract(webAsset.url, webPath, webAsset.sha256));
    }

    await Promise.all(tasks);

    // 8. Save metadata
    const newMeta: Meta = {
      version: targetVersion,
      channel: config.channel,
      publishedAt: entry.published_at,
      native: {
        platform,
        url: nativeAsset.url,
        sha256: nativeAsset.sha256,
      },
      web: webAsset
        ? {
            url: webAsset.url,
            sha256: webAsset.sha256,
          }
        : null,
      downloadedAt: new Date().toISOString(),
    };
    await saveMeta(metaPath, newMeta);

    // 9. Done
    consola.success(`Engine ${targetVersion} installed successfully!`);
    consola.info(`  Native: ${nativePath}`);
    if (webAsset) {
      consola.info(`  Web:    ${webPath}`);
    }
  },
});
