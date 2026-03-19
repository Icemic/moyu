/**
 * Shared utilities for engine management.
 *
 * Provides config loading, platform detection, version metadata,
 * and download/extraction helpers. Migrated from the framework's
 * `scripts/lib/engine-utils.ts`.
 */

import { createHash } from 'node:crypto';
import { createReadStream, createWriteStream, existsSync, readFileSync } from 'node:fs';
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { pipeline } from 'node:stream/promises';
import { createZstdDecompress } from 'node:zlib';
import consola from 'consola';
import pc from 'picocolors';
import { extract as tarExtract } from 'tar';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

export const DEFAULT_CDN_URL = 'https://cdn.momoyu.ink/releases/versions.json';
export const DEFAULT_CHANNEL = 'dev';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** `moyu.json` -> `engine` section */
export interface EngineConfig {
  cdnUrl: string;
  version: string | null;
  channel: string;
}

/** Top-level CDN versions.json */
export interface VersionsJson {
  schema_version: number;
  channels: Record<string, Channel>;
}

export interface Channel {
  latest: string;
  versions: Record<string, VersionEntry>;
}

export interface VersionEntry {
  published_at: string;
  assets: Record<string, Asset>;
}

export interface Asset {
  url: string;
  sha256: string;
  size: number;
}

/** Local metadata stored at `.moyu/engine/meta.json` */
export interface Meta {
  version: string;
  channel: string;
  publishedAt: string;
  native: {
    platform: string;
    url: string;
    sha256: string;
  };
  web: {
    url: string;
    sha256: string;
  } | null;
  downloadedAt: string;
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/**
 * Read the `engine` section from `<projectRoot>/moyu.json`.
 * Returns sensible defaults when the file or section is absent.
 */
export function loadConfig(projectRoot: string): EngineConfig {
  const configPath = join(projectRoot, 'moyu.json');
  const defaults: EngineConfig = {
    cdnUrl: DEFAULT_CDN_URL,
    version: null,
    channel: DEFAULT_CHANNEL,
  };

  if (!existsSync(configPath)) return defaults;

  try {
    const raw = JSON.parse(readFileSync(configPath, 'utf-8'));
    const engine = raw?.engine ?? {};
    return {
      cdnUrl: typeof engine.cdnUrl === 'string' ? engine.cdnUrl : defaults.cdnUrl,
      version: typeof engine.version === 'string' ? engine.version : defaults.version,
      channel: typeof engine.channel === 'string' ? engine.channel : defaults.channel,
    };
  } catch {
    consola.warn('Failed to parse moyu.json – using default engine config.');
    return defaults;
  }
}

// ---------------------------------------------------------------------------
// Platform detection
// ---------------------------------------------------------------------------

const PLATFORM_MAP: Record<string, Record<string, string>> = {
  win32: { x64: 'windows-amd64' },
  linux: { x64: 'linux-amd64', arm64: 'linux-aarch64' },
  darwin: { x64: 'macos-amd64', arm64: 'macos-aarch64' },
};

/**
 * Map `process.platform` + `process.arch` to the CDN asset key.
 * Exits with code 1 if the current platform is unsupported.
 */
export function detectPlatform(): string {
  const key = PLATFORM_MAP[process.platform]?.[process.arch];
  if (key) return key;

  const supported = Object.entries(PLATFORM_MAP)
    .flatMap(([os, archs]) => Object.entries(archs).map(([arch, k]) => `  ${os}/${arch} -> ${k}`))
    .join('\n');
  consola.error(`Unsupported platform: ${process.platform}/${process.arch}\nSupported platforms:\n${supported}`);
  process.exit(1);
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

export async function loadMeta(metaFile: string): Promise<Meta | null> {
  if (!existsSync(metaFile)) return null;
  try {
    return JSON.parse(await readFile(metaFile, 'utf-8'));
  } catch {
    return null;
  }
}

export async function saveMeta(metaFile: string, meta: Meta): Promise<void> {
  await mkdir(dirname(metaFile), { recursive: true });
  await writeFile(metaFile, JSON.stringify(meta, null, 2) + '\n');
}

// ---------------------------------------------------------------------------
// Remote fetch
// ---------------------------------------------------------------------------

/**
 * Fetch and parse the CDN `versions.json`.
 * Validates `schema_version` before returning.
 */
export async function fetchVersionsJson(cdnUrl: string): Promise<VersionsJson> {
  const res = await fetch(cdnUrl);
  if (!res.ok) {
    throw new Error(`Failed to fetch versions.json: HTTP ${res.status} ${res.statusText}`);
  }
  const data: VersionsJson = await res.json();
  if (data.schema_version !== 2) {
    throw new Error(
      `Unsupported versions.json schema_version: ${data.schema_version} (expected 2). ` +
        'Please update @momoyu-ink/cli.',
    );
  }
  return data;
}

// ---------------------------------------------------------------------------
// Download & extract
// ---------------------------------------------------------------------------

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/**
 * Download a `.tar.zst` archive, verify its SHA-256 checksum,
 * then decompress (zstd) and extract (tar) into `destDir`.
 *
 * The destination directory is wiped before extraction.
 */
export async function downloadAndExtract(url: string, destDir: string, expectedSha256: string): Promise<void> {
  const tmpFile = join(tmpdir(), `moyu-engine-${Date.now()}.tar.zst`);

  try {
    const res = await fetch(url);
    if (!res.ok || !res.body) {
      throw new Error(`Download failed: HTTP ${res.status} ${res.statusText}`);
    }

    const totalBytes = Number(res.headers.get('content-length') ?? 0);
    let downloadedBytes = 0;

    const writer = createWriteStream(tmpFile);
    const reader = res.body.getReader();

    // Stream to disk while tracking progress
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      writer.write(value);
      downloadedBytes += value.byteLength;
      if (totalBytes > 0) {
        const pct = ((downloadedBytes / totalBytes) * 100).toFixed(0);
        process.stdout.write(
          `\r  ${pc.dim('Downloading...')} ${formatBytes(downloadedBytes)} / ${formatBytes(totalBytes)} ${pc.dim(`(${pct}%)`)}`,
        );
      } else {
        process.stdout.write(`\r  ${pc.dim('Downloading...')} ${formatBytes(downloadedBytes)}`);
      }
    }

    await new Promise<void>((resolve, reject) => {
      writer.end(() => resolve());
      writer.on('error', reject);
    });
    process.stdout.write('\n');

    // SHA-256 verification
    const hash = createHash('sha256');
    await pipeline(createReadStream(tmpFile), hash);
    const actual = hash.digest('hex');
    if (actual !== expectedSha256) {
      throw new Error(`SHA-256 mismatch!\n  Expected: ${expectedSha256}\n  Actual:   ${actual}`);
    }
    consola.success('Checksum verified.');

    // Extract: zstd -> tar
    await rm(destDir, { recursive: true, force: true });
    await mkdir(destDir, { recursive: true });

    const zstdStream = createZstdDecompress();
    const source = createReadStream(tmpFile);

    await pipeline(source, zstdStream, tarExtract({ cwd: destDir }));
    consola.success(`Extracted to ${destDir}`);
  } finally {
    await rm(tmpFile, { force: true }).catch(() => {});
  }
}
