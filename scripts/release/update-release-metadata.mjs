#!/usr/bin/env node

/**
 * Release metadata generator for Moyu engine.
 *
 * Generates `release.json` (per-version manifest) and updates the global
 * `versions.json` index used by the CDN / update checker.
 *
 * Usage:
 *   node update-release-metadata.mjs \
 *     --version <version>            # e.g. "v0.8.0" or "dev-1a2b3c4"
 *     --channel <channel>            # "stable" | "prerelease" | "dev"
 *     --tag <tag>                    # R2 directory name, e.g. "v0.8.0" or "dev"
 *     --artifacts-dir <path>         # directory containing build artifact subdirs
 *     --cdn-base-url <url>           # e.g. "https://cdn.momoyu.ink" (no trailing /)
 *     --output-dir <path>            # where to write release.json & versions.json
 *     [--existing-versions <path>]   # path to previous versions.json (optional)
 *
 * No external dependencies – only Node.js built-ins.
 */

import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

// ---------------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------------

function parseArgs(argv) {
  const args = {};
  for (let i = 2; i < argv.length; i++) {
    const key = argv[i];
    if (key.startsWith('--')) {
      const name = key.slice(2);
      const value = argv[i + 1];
      if (value === undefined || value.startsWith('--')) {
        args[name] = true;
      } else {
        args[name] = value;
        i++;
      }
    }
  }
  return args;
}

const args = parseArgs(process.argv);

const REQUIRED = ['version', 'channel', 'tag', 'artifacts-dir', 'cdn-base-url', 'output-dir'];
for (const key of REQUIRED) {
  if (!args[key]) {
    console.error(`Missing required argument: --${key}`);
    process.exit(1);
  }
}

const VERSION = args['version'];
const CHANNEL = args['channel'];
const TAG = args['tag'];
const ARTIFACTS_DIR = resolve(args['artifacts-dir']);
const CDN_BASE = args['cdn-base-url'].replace(/\/+$/, '');
const OUTPUT_DIR = resolve(args['output-dir']);
const EXISTING_VERSIONS = args['existing-versions'] ? resolve(args['existing-versions']) : null;

if (!['stable', 'prerelease', 'dev'].includes(CHANNEL)) {
  console.error(`Invalid channel "${CHANNEL}". Must be one of: stable, prerelease, dev`);
  process.exit(1);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Compute SHA-256 hex digest of a file. */
function sha256(filePath) {
  const hash = createHash('sha256');
  const data = readFileSync(filePath);
  hash.update(data);
  return hash.digest('hex');
}

/**
 * Scan the artifacts directory for .tar.zst files.
 *
 * Expected layout (produced by actions/download-artifact):
 *   artifacts/
 *     linux-amd64-0.8.0/
 *       linux-amd64-0.8.0.tar.zst
 *     windows-amd64-0.8.0/
 *       windows-amd64-0.8.0.tar.zst
 *     ...
 *
 * Returns an array of { platform, arch, assetKey, filePath, filename }.
 */
function discoverArtifacts(dir) {
  const results = [];
  if (!existsSync(dir)) {
    console.error(`Artifacts directory not found: ${dir}`);
    process.exit(1);
  }

  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    const subdir = join(dir, entry.name);
    for (const file of readdirSync(subdir)) {
      if (!file.endsWith('.tar.zst')) continue;
      const filePath = join(subdir, file);

      // Parse platform-arch from the artifact directory name.
      // Directory name format: {platform}-{arch}-{version}
      // e.g. "linux-amd64-0.8.0", "web-universal-dev"
      const dirName = entry.name;
      const parts = dirName.split('-');
      if (parts.length < 3) {
        console.warn(`Skipping unrecognised artifact directory: ${dirName}`);
        continue;
      }
      const platform = parts[0];
      const arch = parts[1];
      const assetKey = `${platform}-${arch}`;
      // Fixed filename for R2 (no version suffix)
      const filename = `${assetKey}.tar.zst`;

      results.push({ platform, arch, assetKey, filePath, filename });
    }
  }
  return results;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

function main() {
  console.log(`Generating release metadata for ${VERSION} (channel: ${CHANNEL}, tag: ${TAG})`);

  const artifacts = discoverArtifacts(ARTIFACTS_DIR);
  if (artifacts.length === 0) {
    console.error('No .tar.zst artifacts found.');
    process.exit(1);
  }
  console.log(`Found ${artifacts.length} artifact(s):`);

  // Build asset entries
  const assets = {};
  for (const art of artifacts) {
    const hash = sha256(art.filePath);
    const size = statSync(art.filePath).size;
    const url = `${CDN_BASE}/releases/${TAG}/${art.filename}`;
    assets[art.assetKey] = { filename: art.filename, url, sha256: hash, size };
    console.log(`  ${art.assetKey}: ${art.filename} (${size} bytes, sha256=${hash.slice(0, 12)}...)`);
  }

  const now = new Date().toISOString();

  // -- release.json (per-version manifest) ----------------------------------
  const releaseJson = {
    schema_version: 2,
    version: VERSION,
    channel: CHANNEL,
    published_at: now,
    assets,
  };

  mkdirSync(OUTPUT_DIR, { recursive: true });
  const releaseJsonPath = join(OUTPUT_DIR, 'release.json');
  writeFileSync(releaseJsonPath, JSON.stringify(releaseJson, null, 2) + '\n');
  console.log(`Wrote ${releaseJsonPath}`);

  // -- versions.json (global index) -----------------------------------------
  let versions = { schema_version: 2, channels: {} };

  if (EXISTING_VERSIONS && existsSync(EXISTING_VERSIONS)) {
    try {
      const raw = readFileSync(EXISTING_VERSIONS, 'utf-8');
      const parsed = JSON.parse(raw);
      if (parsed && parsed.schema_version === 2 && parsed.channels) {
        versions = parsed;
        console.log('Loaded existing versions.json');
      } else {
        console.warn('Existing versions.json has unexpected format, starting fresh.');
      }
    } catch (err) {
      console.warn(`Failed to parse existing versions.json: ${err.message}. Starting fresh.`);
    }
  }

  // Ensure channel object exists
  if (!versions.channels[CHANNEL]) {
    versions.channels[CHANNEL] = { latest: null, versions: {} };
  }
  const ch = versions.channels[CHANNEL];

  // Build the version entry (subset of release.json, without schema_version/channel)
  const versionEntry = {
    published_at: now,
    assets: {},
  };
  for (const [key, asset] of Object.entries(assets)) {
    versionEntry.assets[key] = {
      url: asset.url,
      sha256: asset.sha256,
      size: asset.size,
    };
  }

  if (CHANNEL === 'dev') {
    // Dev channel: replace entirely (only keep latest)
    ch.latest = VERSION;
    ch.versions = { [VERSION]: versionEntry };
  } else {
    // Stable / prerelease: add version entry, update latest pointer
    ch.versions[VERSION] = versionEntry;
    ch.latest = VERSION;
  }

  const versionsJsonPath = join(OUTPUT_DIR, 'versions.json');
  writeFileSync(versionsJsonPath, JSON.stringify(versions, null, 2) + '\n');
  console.log(`Wrote ${versionsJsonPath}`);

  console.log('Done.');
}

main();
