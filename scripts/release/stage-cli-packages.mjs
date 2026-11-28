#!/usr/bin/env node

/**
 * Stage @momoyu-ink/cli and its 5 platform-specific binary packages for npm publishing.
 *
 * Usage:
 *   node scripts/release/stage-cli-packages.mjs \
 *     --version 0.10.0 \
 *     --artifacts-dir ./devtool-artifacts \
 *     --output-dir ./npm-stage
 */

import { chmodSync, cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const PLATFORMS = [
  { key: 'win32-x64', pkg: '@momoyu-ink/cli-win32-x64', os: 'win32', cpu: 'x64', bin: 'moyu-devtool.exe' },
  { key: 'linux-x64', pkg: '@momoyu-ink/cli-linux-x64', os: 'linux', cpu: 'x64', bin: 'moyu-devtool' },
  { key: 'linux-arm64', pkg: '@momoyu-ink/cli-linux-arm64', os: 'linux', cpu: 'arm64', bin: 'moyu-devtool' },
  { key: 'darwin-x64', pkg: '@momoyu-ink/cli-darwin-x64', os: 'darwin', cpu: 'x64', bin: 'moyu-devtool' },
  { key: 'darwin-arm64', pkg: '@momoyu-ink/cli-darwin-arm64', os: 'darwin', cpu: 'arm64', bin: 'moyu-devtool' },
];

function parseArgs(argv) {
  const args = { allowMissing: false };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === '--version') args.version = argv[++i];
    else if (arg === '--artifacts-dir') args.artifactsDir = argv[++i];
    else if (arg === '--output-dir') args.outputDir = argv[++i];
    else if (arg === '--allow-missing') args.allowMissing = true;
    else throw new Error(`Unknown argument: ${arg}`);
  }
  if (!args.version) throw new Error('--version is required');
  if (!args.artifactsDir) throw new Error('--artifacts-dir is required');
  if (!args.outputDir) throw new Error('--output-dir is required');
  return args;
}

const args = parseArgs(process.argv.slice(2));
const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, '..', '..');
const cliDir = join(repoRoot, 'packages', 'cli');
const artifactsDir = resolve(args.artifactsDir);
const outputDir = resolve(args.outputDir);

if (existsSync(outputDir)) {
  rmSync(outputDir, { recursive: true, force: true });
}
mkdirSync(outputDir, { recursive: true });

const optionalDependencies = {};

for (const plat of PLATFORMS) {
  optionalDependencies[plat.pkg] = args.version;

  const srcBin = join(artifactsDir, plat.key, plat.bin);
  if (!existsSync(srcBin)) {
    if (args.allowMissing) {
      console.warn(`Skipping missing binary for ${plat.key}: ${srcBin}`);
      continue;
    }
    throw new Error(`Missing built binary for ${plat.key}: ${srcBin}`);
  }

  const pkgDir = join(outputDir, 'platforms', plat.key);
  const binDir = join(pkgDir, 'bin');
  mkdirSync(binDir, { recursive: true });

  const dstBin = join(binDir, plat.bin);
  cpSync(srcBin, dstBin);
  if (plat.os !== 'win32') {
    chmodSync(dstBin, 0o755);
  }

  const platformPkgJson = {
    name: plat.pkg,
    version: args.version,
    description: `Prebuilt moyu-devtool binary for ${plat.os}-${plat.cpu}`,
    license: 'MPL-2.0',
    os: [plat.os],
    cpu: [plat.cpu],
    files: ['bin'],
    publishConfig: {
      access: 'public',
    },
  };
  writeFileSync(join(pkgDir, 'package.json'), `${JSON.stringify(platformPkgJson, null, 2)}\n`);
  console.log(`Staged ${plat.pkg}@${args.version}`);
}

const mainOutDir = join(outputDir, 'cli');
mkdirSync(mainOutDir, { recursive: true });
cpSync(join(cliDir, 'bin'), join(mainOutDir, 'bin'), { recursive: true });
cpSync(join(cliDir, 'README.md'), join(mainOutDir, 'README.md'));

const basePkg = JSON.parse(readFileSync(join(cliDir, 'package.json'), 'utf-8'));
const stagedMainPkg = {
  ...basePkg,
  version: args.version,
  optionalDependencies,
};
writeFileSync(join(mainOutDir, 'package.json'), `${JSON.stringify(stagedMainPkg, null, 2)}\n`);
console.log(`Staged ${stagedMainPkg.name}@${args.version} with ${Object.keys(optionalDependencies).length} optionalDependencies`);
